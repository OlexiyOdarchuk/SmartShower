#include <Arduino.h>
#include <secrets.hpp>
#include <FastBot2.h>
#include <WiFi.h>
#include <Ticker.h>
#include <bot.hpp>
#include <Shower.hpp>
#include <SmartShower.hpp>
#include <CircularBuffer.hpp>
#include <GyverOLED.h>
#include <logic.hpp>
#include <NTPClient.h>

// — константи —
namespace {
constexpr uint16_t TEMP_DEBOUNCE_MS   = 30;
constexpr uint16_t TEMP_REFIRE_MS     = 300;
constexpr uint16_t QUEUE_BTN_REFIRE_MS = 2000;
constexpr uint16_t ERROR_DISPLAY_MS   = 2000;
constexpr uint16_t OLED_REFRESH_MS    = 500;
constexpr uint16_t ANIM_FRAME_MS      = 100;
constexpr uint32_t WORKING_TIME_TTL_MS = 1000; // як часто перепитуємо NTP

constexpr int TEMP_PINS[2][4] = {
    {SHOWER_1_TEMPERATURE_BUTTON_1, SHOWER_1_TEMPERATURE_BUTTON_2,
     SHOWER_1_TEMPERATURE_BUTTON_3, SHOWER_1_TEMPERATURE_BUTTON_4},
    {SHOWER_2_TEMPERATURE_BUTTON_1, SHOWER_2_TEMPERATURE_BUTTON_2,
     SHOWER_2_TEMPERATURE_BUTTON_3, SHOWER_2_TEMPERATURE_BUTTON_4},
};

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

    pinMode(BUZZER, OUTPUT);
    digitalWrite(BUZZER, LOW);
    // Не викликаємо noTone() — LEDC канал ще не створено.

    shower1.init();
    shower2.init();
    oled.init();
    oled.setScale(1);

    pinMode(QUEUE_BUTTON, INPUT_PULLUP);
    pinMode(SHOWER_1_BUTTON, INPUT_PULLUP);
    pinMode(SHOWER_2_BUTTON, INPUT_PULLUP);

    // Дамо підтяжкам час встановитися (RC settling).
    delay(10);

    // Діагностика: показуємо стартові стани темп-кнопок.
    for (uint8_t s = 0; s < 2; s++)
    {
        for (uint8_t c = 0; c < 4; c++)
        {
            int raw = digitalRead(TEMP_PINS[s][c]);
            Serial.printf("[Init] S%u T%u pin=%d -> %s\n",
                          s + 1, c + 1, TEMP_PINS[s][c],
                          raw == HIGH ? "HIGH (idle)" : "LOW (stuck/pressed)");
        }
    }
    // Усі поля TempButtonState уже false/0 за замовчанням; явно не seed-имо —
    // інакше «застряглі LOW» піни сприймаються як уже-натиснуті і не дають
    // реальним натисканням фронту.

    lastShower1ButtonState = (digitalRead(SHOWER_1_BUTTON) == LOW);
    lastShower2ButtonState = (digitalRead(SHOWER_2_BUTTON) == LOW);
    lastQueueButtonState   = (digitalRead(QUEUE_BUTTON)    == LOW);
    lastShower1Busy        = shower1.isBusyNow();
    lastShower2Busy        = shower2.isBusyNow();

    Serial.println("--- SmartShower init done ---");
}

// ─────────────────────────── working time ───────────────────────────

bool SmartShower::computeWorkingTime()
{
    timeClient.update();
    // Failsafe: поки NTP не синхронізувався, вважаємо що зараз робочий час —
    // інакше getHours() == 0 і система постійно «нічна», блокуючи функціонал.
    if (!timeClient.isTimeSet()) return true;
    uint8_t hour = timeClient.getHours();
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
    if (now - workingTimeUpdatedAt < WORKING_TIME_TTL_MS && workingTimeUpdatedAt != 0) return;
    workingTimeCached = computeWorkingTime();
    workingTimeUpdatedAt = now;
}

bool SmartShower::isWorkingTime()
{
    // Боту OK повертати кешоване значення — затримка до 1с не критична.
    return workingTimeCached;
}

// ─────────────────────────── temp buttons ───────────────────────────

void SmartShower::handleTempButton(uint8_t s, uint8_t c, int pin, Shower &shower)
{
    TempButtonState &st = tempBtn[s][c];
    ulong now = millis();
    bool pressed = (digitalRead(pin) == LOW);

    if (pressed != st.raw)
    {
        // ДІАГНОСТИКА: лог сирого фронту, незалежно від дебаунсу
        Serial.printf("[DBG] pin=%d (S%u T%u): %s -> %s\n",
                      pin, s + 1, c + 1,
                      st.raw ? "LOW" : "HIGH",
                      pressed ? "LOW" : "HIGH");
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
        shower.setWaterTemperature(c + 1);
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
        handleTempButton(0, c, TEMP_PINS[0][c], shower1);
        handleTempButton(1, c, TEMP_PINS[1][c], shower2);
    }
}

// ─────────────────────────── queue API ───────────────────────────

bool SmartShower::addingToQueue(const String &id, const String &name)
{
    if (!isWorkingTime()) return false;
    QueueLock lock(queueMutex);
    if (queue.isFull()) return false;
    queue.push({id, name});
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

void SmartShower::queueReduction(const String &id)
{
    QueueLock lock(queueMutex);
    int8_t idx = -1;
    for (uint8_t i = 0; i < queue.size(); i++)
        if (queue[i].id == id) { idx = i; break; }
    queueReductionByIndexUnlocked(idx);
}

int8_t SmartShower::isInQueue(const String &id)
{
    QueueLock lock(queueMutex);
    for (uint8_t i = 0; i < queue.size(); i++)
        if (queue[i].id == id) return i;
    return -1;
}

QueueEntry SmartShower::getQueueAt(uint8_t index)
{
    QueueLock lock(queueMutex);
    if (index >= queue.size()) return {};
    return queue[index];
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
}

void SmartShower::clearQueueIfNonWorkingTime()
{
    if (!isWorkingTime()) clearQueue();
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
            String takenId;
            {
                QueueLock lock(queueMutex);
                if (!queue.isEmpty())
                {
                    QueueEntry entry = queue.shift();
                    takenId = entry.id;
                    tookFromQueue = true;
                }
            }
            if (tookFromQueue)
            {
                shower.setWhoNow(takenId);
                buzzerBeep(false);
                pendingNotifyNext = true; // бот сповістить наступного
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
    // Викликається з бот-задачі (core 0). Просто ставимо прапор; саме пищання
    // зробить core 1 у processPendingBeep(), щоб всі tone()/Ticker виклики
    // ішли з одного потоку.
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
    lastOledRender = ""; // анімація — кеш скидаємо
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

    // Темп-кнопки і релізи душів обробляємо незалежно від часу доби —
    // якщо хтось у душі вночі, він має право натиснути температуру.
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
