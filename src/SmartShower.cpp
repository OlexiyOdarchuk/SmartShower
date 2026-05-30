#include <Arduino.h>
#include <secrets.hpp>
#include <FastBot2.h>
#include <WiFi.h>
#include <Ticker.h>
#include <Preferences.h>
#include <bot.hpp>
#include <Shower.hpp>
#include <SmartShower.hpp>
#include <CircularBuffer.hpp>
#include <GyverOLED.h>
#include <logic.hpp>
#include <timeutil.hpp>

namespace {
constexpr uint16_t TEMP_DEBOUNCE_MS    = 30;
constexpr uint16_t TEMP_REFIRE_MS      = 300;
constexpr uint16_t QUEUE_BTN_REFIRE_MS = 2000;
constexpr uint16_t ERROR_DISPLAY_MS    = 2000;
constexpr uint16_t OLED_REFRESH_MS     = 500;
constexpr uint16_t ANIM_FRAME_MS       = 100;
constexpr uint32_t WORKING_TIME_TTL_MS = 1000;
constexpr uint32_t AUTO_REMOVE_MS      = 5UL * 60UL * 1000UL; // 5 хв на «з'явитися в душі»

// Роздільники для серіалізації черги в NVS (Unit Separator / Record Separator,
// гарантовано не зустрічаються в username/first_name).
constexpr char FIELD_SEP_CH  = '\x1F';
constexpr char RECORD_SEP_CH = '\x1E';
constexpr const char *NVS_NAMESPACE = "ss";
constexpr const char *NVS_QUEUE_KEY = "queue";

constexpr int TEMP_PINS[2][4] = {
    {SHOWER_1_TEMPERATURE_BUTTON_1, SHOWER_1_TEMPERATURE_BUTTON_2,
     SHOWER_1_TEMPERATURE_BUTTON_3, SHOWER_1_TEMPERATURE_BUTTON_4},
    {SHOWER_2_TEMPERATURE_BUTTON_1, SHOWER_2_TEMPERATURE_BUTTON_2,
     SHOWER_2_TEMPERATURE_BUTTON_3, SHOWER_2_TEMPERATURE_BUTTON_4},
};

constexpr bool isBadTempPin(int pin) { return pin == 2 || pin == 34 || pin == 35; }

void buzzerOffCallback() { noTone(BUZZER); }

class QueueLock
{
public:
    explicit QueueLock(SemaphoreHandle_t m) : m_(m)
    {
        if (m_) xSemaphoreTake(m_, portMAX_DELAY);
    }
    ~QueueLock() { if (m_) xSemaphoreGive(m_); }
private:
    SemaphoreHandle_t m_;
};
} // namespace

SmartShower::SmartShower(FastBot2 &botRef,
                         Shower &shower1Ref, Shower &shower2Ref,
                         GyverOLED<SSD1306_128x32, OLED_NO_BUFFER> &oledRef)
    : bot(botRef), shower1(shower1Ref), shower2(shower2Ref), oled(oledRef)
{
}

// ─────────────────────────── init ───────────────────────────

void SmartShower::init()
{
    Serial.println("--- SmartShower init ---");
    queueMutex = xSemaphoreCreateMutex();
    prefs.begin(NVS_NAMESPACE, false); // RW

    pinMode(BUZZER, OUTPUT);
    digitalWrite(BUZZER, LOW);

    // Прокидаємо LEDC канал заздалегідь, щоб перший noTone() не лаявся.
    tone(BUZZER, 1, 1);
    delay(2);
    noTone(BUZZER);

    shower1.init();
    shower2.init();
    oled.init();
    oled.setScale(1);

    pinMode(QUEUE_BUTTON, INPUT_PULLUP);
    pinMode(SHOWER_1_BUTTON, INPUT_PULLUP);
    pinMode(SHOWER_2_BUTTON, INPUT_PULLUP);

    // PinMode для робочих темп-пінів. Зламані просто пропускаємо.
    for (uint8_t s = 0; s < 2; s++)
    {
        for (uint8_t c = 0; c < 4; c++)
        {
            int p = TEMP_PINS[s][c];
            if (!isBadTempPin(p)) pinMode(p, INPUT_PULLUP);
        }
    }

    delay(10); // дамо підтяжкам встановитися

    // Діагностика темп-пінів при старті.
    for (uint8_t s = 0; s < 2; s++)
    {
        for (uint8_t c = 0; c < 4; c++)
        {
            int p = TEMP_PINS[s][c];
            if (isBadTempPin(p))
            {
                Serial.printf("[Init] S%u T%u pin=%d -> DISABLED\n", s + 1, c + 1, p);
                continue;
            }
            int raw = digitalRead(p);
            Serial.printf("[Init] S%u T%u pin=%d -> %s\n",
                          s + 1, c + 1, p,
                          raw == HIGH ? "HIGH (idle)" : "LOW (pressed?)");
        }
    }

    lastShower1ButtonState = (digitalRead(SHOWER_1_BUTTON) == LOW);
    lastShower2ButtonState = (digitalRead(SHOWER_2_BUTTON) == LOW);
    lastQueueButtonState   = (digitalRead(QUEUE_BUTTON)    == LOW);
    lastShower1Busy        = shower1.isBusyNow();
    lastShower2Busy        = shower2.isBusyNow();

    restoreQueueOnBoot();

    Serial.println("--- SmartShower init done ---");
}

// ─────────────────────────── persistence ───────────────────────────

void SmartShower::persistQueueUnlocked()
{
    // Очікує, що queueMutex уже взято.
    String s;
    s.reserve(queue.size() * 32);
    for (uint8_t i = 0; i < queue.size(); i++)
    {
        if (i > 0) s += RECORD_SEP_CH;
        s += queue[i].id;
        s += FIELD_SEP_CH;
        s += queue[i].name;
    }
    prefs.putString(NVS_QUEUE_KEY, s);
}

void SmartShower::restoreQueueOnBoot()
{
    String s = prefs.getString(NVS_QUEUE_KEY, "");
    if (s.length() == 0)
    {
        Serial.println("[NVS] queue empty");
        return;
    }
    int start = 0;
    uint8_t count = 0;
    while (start < (int)s.length())
    {
        int rec = s.indexOf(RECORD_SEP_CH, start);
        if (rec < 0) rec = s.length();
        int sep = s.indexOf(FIELD_SEP_CH, start);
        if (sep < 0 || sep > rec) break;
        String id   = s.substring(start, sep);
        String name = s.substring(sep + 1, rec);
        if (id.length() > 0 && !queue.isFull())
        {
            queue.push({id, name});
            count++;
        }
        start = rec + 1;
    }
    Serial.printf("[NVS] restored %u queue entries\n", count);
}

// ─────────────────────────── working time ───────────────────────────

bool SmartShower::computeWorkingTime()
{
    if (!timeutil::isSet()) return true; // failsafe доки NTP не синхронізувався
    uint8_t hour = timeutil::hour();
    auto inRange = [hour](uint8_t start, uint8_t finish) {
        return (start <= finish) ? (hour >= start && hour <= finish)
                                 : (hour >= start || hour <= finish);
    };
    bool night  = inRange(NIGHT_TIME_START,  NIGHT_TIME_FINISH);
    bool midday = inRange(MIDDAY_TIME_START, MIDDAY_TIME_FINISH);
    return !night && !midday;
}

void SmartShower::refreshWorkingTime()
{
    ulong now = millis();
    if (workingTimeUpdatedAt != 0 && now - workingTimeUpdatedAt < WORKING_TIME_TTL_MS) return;
    workingTimeCached = computeWorkingTime();
    workingTimeUpdatedAt = now;
}

bool SmartShower::isWorkingTime() { return workingTimeCached; }

String SmartShower::infoReport()
{
    // Зчитуємо температуру обох душів і довжину черги під одним блокуванням,
    // щоб не зчитати String у момент перезапису з core 1 (temp-кнопки).
    QueueLock lock(queueMutex);
    String body;
    body.reserve(220);
    body  = "📊 Загальна інформація:\n\n";
    body += "🚿 Душ 1:\n" + shower1.getWaterTemperature() + "\n\n";
    body += "🚿 Душ 2:\n" + shower2.getWaterTemperature() + "\n\n";
    body += "Черга: " + String(queue.size());
    return body;
}

// ─────────────────────────── temp buttons ───────────────────────────

void SmartShower::handleTempButton(uint8_t s, uint8_t c, int pin, Shower &shower)
{
    TempButtonState &st = tempBtn[s][c];
    ulong now = millis();
    bool pressed = (digitalRead(pin) == LOW);

    if (pressed != st.raw)
    {
        st.raw = pressed;
        st.changeAt = now;
        return;
    }
    if (now - st.changeAt < TEMP_DEBOUNCE_MS) return;
    if (pressed == st.stable) return;

    st.stable = pressed;
    if (!pressed) return;

    if (now - st.firedAt < TEMP_REFIRE_MS) return;
    st.firedAt = now;

    Serial.printf("[Input] S%u T%u pressed\n", s + 1, c + 1);
    if (shower.getWhoNow().length() > 0)
    {
        {
            // waterTemperature читається з core 0 у infoReport() — серіалізуємо доступ.
            QueueLock lock(queueMutex);
            shower.setWaterTemperature(c + 1);
        }
        buzzerBeep(false);
        showErrorOnOled("S" + String(s + 1) + " T=" + String(c + 1));
    }
    else
    {
        buzzerBeep(true);
        showErrorOnOled("S" + String(s + 1) + ": немає юзера");
    }
}

void SmartShower::updateTemperatureButtons()
{
    for (uint8_t c = 0; c < 4; c++)
    {
        if (!isBadTempPin(TEMP_PINS[0][c]))
            handleTempButton(0, c, TEMP_PINS[0][c], shower1);
        if (!isBadTempPin(TEMP_PINS[1][c]))
            handleTempButton(1, c, TEMP_PINS[1][c], shower2);
    }
}

// ─────────────────────────── queue API ───────────────────────────

JoinResult SmartShower::tryJoin(const String &id, const String &name)
{
    if (!isWorkingTime()) return {JoinResult::OFF_HOURS, 0};
    QueueLock lock(queueMutex);
    for (uint8_t i = 0; i < queue.size(); i++)
        if (queue[i].id == id) return {JoinResult::ALREADY_IN, i + 1};
    if (queue.isFull()) return {JoinResult::FULL, 0};
    queue.push({id, name});
    persistQueueUnlocked();
    return {JoinResult::ADDED, (int)queue.size()};
}

bool SmartShower::leaveQueue(const String &id, bool &wasFirstOut)
{
    QueueLock lock(queueMutex);
    int8_t idx = -1;
    for (uint8_t i = 0; i < queue.size(); i++)
        if (queue[i].id == id) { idx = i; break; }
    if (idx == -1) { wasFirstOut = false; return false; }
    wasFirstOut = (idx == 0);
    queueReductionByIndexUnlocked(idx);
    persistQueueUnlocked();
    // Авто-вибуття (firstNotifyId/At) живе виключно на core 1 (runAutoRemoveTick):
    // наступний тік сам помітить зміну голови черги і переарганує таймер.
    return true;
}

void SmartShower::queueReductionByIndexUnlocked(int8_t index)
{
    int size = queue.size();
    if (index < 0 || index >= size) return;
    QueueEntry temp[30];
    for (int i = 0; i < size; i++) temp[i] = queue.shift();
    for (int i = 0; i < size; i++) if (i != index) queue.push(temp[i]);
}

int8_t SmartShower::isInQueue(const String &id)
{
    QueueLock lock(queueMutex);
    for (uint8_t i = 0; i < queue.size(); i++)
        if (queue[i].id == id) return i;
    return -1;
}

uint8_t SmartShower::snapshotQueue(QueueEntry *out, uint8_t maxOut)
{
    QueueLock lock(queueMutex);
    uint8_t count = queue.size();
    if (count > maxOut) count = maxOut;
    for (uint8_t i = 0; i < count; i++) out[i] = queue[i];
    return count;
}

QueueHead SmartShower::getHead()
{
    QueueLock lock(queueMutex);
    if (queue.isEmpty()) return {"", "", true};
    const QueueEntry &first = queue.first();
    return { first.id, first.displayName(), false };
}

uint8_t SmartShower::queueLen()
{
    QueueLock lock(queueMutex);
    return queue.size();
}

void SmartShower::clearQueue()
{
    QueueLock lock(queueMutex);
    while (!queue.isEmpty()) queue.shift();
    persistQueueUnlocked();
    // firstNotifyId/At скине runAutoRemoveTick на core 1, побачивши порожню голову.
}

void SmartShower::clearQueueIfNonWorkingTime()
{
    if (!isWorkingTime()) clearQueue();
}

void SmartShower::runAutoRemoveTick()
{
    // Викликається ВИКЛЮЧНО з core 1 (loop), тому firstNotifyId/firstNotifyAt
    // тут — приватний стан одного ядра і не потребує блокування (блокуємо лише
    // саму чергу). Відлік таймауту йде лише доки є вільна кабінка: якщо обидва
    // душі зайняті, людина об'єктивно не може зайти, і виганяти її не можна.
    QueueHead head = getHead(); // бере queueMutex усередині
    bool anyShowerFree = !shower1.isBusyNow() || !shower2.isBusyNow();
    ulong now = millis();

    // Немає реального «наступного» (порожньо або анонімна кнопка) — скидаємо.
    if (head.isEmpty || head.id == "0")
    {
        firstNotifyId = "";
        firstNotifyAt = 0;
        return;
    }

    // Новий перший у черзі — починаємо стежити за ним.
    if (head.id != firstNotifyId)
    {
        firstNotifyId = head.id;
        firstNotifyAt = anyShowerFree ? now : 0;
        return;
    }

    // Той самий перший: керуємо відліком залежно від наявності вільної кабінки.
    if (!anyShowerFree)
    {
        firstNotifyAt = 0; // пауза, поки нікуди заходити
        return;
    }
    if (firstNotifyAt == 0)
    {
        firstNotifyAt = now; // душ щойно звільнився — даємо повний таймаут
        return;
    }
    if (now - firstNotifyAt < AUTO_REMOVE_MS) return;

    // Таймаут вийшов: вільний душ був, але людина не зайшла — виганяємо.
    Serial.printf("[Queue] auto-removing %s after %lus timeout\n",
                  firstNotifyId.c_str(), AUTO_REMOVE_MS / 1000);
    {
        QueueLock lock(queueMutex);
        if (!queue.isEmpty() && queue.first().id == firstNotifyId)
        {
            queueReductionByIndexUnlocked(0);
            persistQueueUnlocked();
        }
    }
    firstNotifyId = "";
    firstNotifyAt = 0;
    pendingNotifyNext = true; // сповістити нового першого
}

// ─────────────────────────── physical buttons ───────────────────────────

void SmartShower::pressQueueButton()
{
    if (!isWorkingTime())
    {
        buzzerBeep(true);
        showErrorOnOled("Не робочий час");
        return;
    }
    bool added = false;
    {
        QueueLock lock(queueMutex);
        if (!queue.isFull())
        {
            queue.push({"0", ""});
            persistQueueUnlocked();
            added = true;
        }
    }
    if (added) buzzerBeep(false);
    else
    {
        buzzerBeep(true);
        showErrorOnOled("Черга повна");
    }
}

void SmartShower::handleShowerButton(Shower &shower, bool &lastState, int buttonPin)
{
    bool pressed = (digitalRead(buttonPin) == LOW);
    if (pressed && !lastState)
    {
        if (!isWorkingTime())
        {
            buzzerBeep(true);
            showErrorOnOled("Не робочий час");
        }
        else
        {
            bool tookFromQueue = false;
            String takenName;
            {
                QueueLock lock(queueMutex);
                if (!queue.isEmpty())
                {
                    QueueEntry entry = queue.shift();
                    // Зберігаємо читабельне ім'я (для /get_info), а не сирий id.
                    takenName = (entry.id == "0") ? String("0") : entry.displayName();
                    tookFromQueue = true;
                    persistQueueUnlocked();
                }
            }
            if (tookFromQueue)
            {
                shower.setWhoNow(takenName);
                buzzerBeep(false);
                // Голова черги змінилась — сповістити нового першого. Авто-вибуття
                // переарганує runAutoRemoveTick наступним тіком (core 1).
                pendingNotifyNext = true;
            }
            else
            {
                shower.setWhoNow("0");
                buzzerBeep(false);
            }
        }
    }
    lastState = pressed;
    shower.updateDisplay();
}

void SmartShower::handleQueueButton()
{
    bool pressed = (digitalRead(QUEUE_BUTTON) == LOW);
    if (pressed && !lastQueueButtonState)
    {
        if (millis() - lastQueueButtonPress >= QUEUE_BTN_REFIRE_MS)
        {
            pressQueueButton();
            lastQueueButtonPress = millis();
        }
        else
        {
            buzzerBeep(true);
            showErrorOnOled("Зачекайте...");
        }
    }
    lastQueueButtonState = pressed;
}

void SmartShower::detectShowerRelease()
{
    bool s1 = shower1.isBusyNow();
    bool s2 = shower2.isBusyNow();
    if (lastShower1Busy && !s1) shower1.clearWhoNow();
    if (lastShower2Busy && !s2) shower2.clearWhoNow();
    lastShower1Busy = s1;
    lastShower2Busy = s2;
}

// ─────────────────────────── buzzer ───────────────────────────

void SmartShower::buzzerBeep(bool isError)
{
    Serial.printf("[Buzzer] %s\n", isError ? "ERROR" : "OK");
    int freq      = isError ? 400 : 1000;
    uint32_t dur  = isError ? 200 : 80;
    tone(BUZZER, freq);
    buzzerOffTicker.once_ms(dur, buzzerOffCallback);
}

void SmartShower::requestBeep(bool isError)
{
    pendingBeepIsError = isError;
    pendingBeep = true;
}

void SmartShower::processPendingBeep()
{
    if (!pendingBeep) return;
    bool isError = pendingBeepIsError;
    pendingBeep = false;
    buzzerBeep(isError);
}

// ─────────────────────────── OLED ───────────────────────────

void SmartShower::renderOled(const String &content)
{
    if (content == lastOledRender) return;
    lastOledRender = content;
    oled.clear();
    oled.setScale(1);
    int nl = content.indexOf('\n');
    if (nl < 0)
    {
        oled.setCursor(0, 0);
        oled.print(content);
    }
    else
    {
        oled.setCursor(0, 0);
        oled.print(content.substring(0, nl));
        oled.setCursor(0, 2);
        oled.print(content.substring(nl + 1));
    }
    oled.update();
}

void SmartShower::queueDisplay()
{
    QueueHead head = getHead();
    String content;
    content.reserve(48);
    content  = "Черга: " + String(queueLen());
    content += "\n";
    content += head.isEmpty ? String("Empty") : ("Next: " + head.displayName);
    renderOled(content);
}

void SmartShower::showErrorOnOled(const String &error)
{
    errorMessage = error;
    errorDisplayStart = millis();
}

void SmartShower::showNonWorkingTimeAnimation()
{
    if (millis() - lastAnimationUpdate < ANIM_FRAME_MS) return;
    lastAnimationUpdate = millis();
    lastOledRender = "";
    oled.clear();
    oled.setScale(2);
    oled.setCursor(animationOffset, 1);
    oled.print("OFF TIME");
    oled.update();
    animationOffset -= 3;
    if (animationOffset < -100) animationOffset = 128;
}

// ─────────────────────────── run ───────────────────────────

void SmartShower::run()
{
    refreshWorkingTime();
    processPendingBeep();

    // Темп-кнопки і релізи душів обробляємо незалежно від часу доби.
    updateTemperatureButtons();
    detectShowerRelease();

    if (!workingTimeCached)
    {
        clearQueueIfNonWorkingTime();
        showNonWorkingTimeAnimation();
        shower1.updateDisplay();
        shower2.updateDisplay();
        return;
    }

    handleShowerButton(shower1, lastShower1ButtonState, SHOWER_1_BUTTON);
    handleShowerButton(shower2, lastShower2ButtonState, SHOWER_2_BUTTON);
    handleQueueButton();
    runAutoRemoveTick();

    if (errorMessage.length() > 0 && millis() - errorDisplayStart < ERROR_DISPLAY_MS)
    {
        renderOled("!\n" + errorMessage);
    }
    else
    {
        errorMessage = "";
        if (millis() - lastOledUpdate > OLED_REFRESH_MS)
        {
            queueDisplay();
            lastOledUpdate = millis();
        }
    }
}
