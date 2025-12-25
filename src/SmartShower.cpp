#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <Shower.hpp>
#include <SmartShower.hpp>
#include <CircularBuffer.hpp>
#include <GyverOLED.h>
#include <logic.hpp>
#include <NTPClient.h>

SmartShower::SmartShower(FastBot2 &botRef, Shower &shower1Ref, Shower &shower2Ref, GyverOLED<SSD1306_128x32, OLED_NO_BUFFER> &oledRef, u8_t temperatureButton1, u8_t temperatureButton2, u8_t temperatureButton3, u8_t temperatureButton4) : bot(botRef), shower1(shower1Ref), shower2(shower2Ref), oled(oledRef), temperatureButtons{temperatureButton1, temperatureButton2, temperatureButton3, temperatureButton4}
{
    temperatureGrounds[0] = shower1.getTemperatureGround();
    temperatureGrounds[1] = shower2.getTemperatureGround();
    oled.init();
    for (int r = 0; r < 2; r++)
    {
        pinMode(temperatureGrounds[r], OUTPUT);
        digitalWrite(temperatureGrounds[r], HIGH);
    }

    for (int c = 0; c < 4; c++)
    {
        pinMode(temperatureButtons[c], INPUT_PULLUP);
    }

    pinMode(QUEUE_BUTTON, INPUT_PULLUP);
    pinMode(BUZZER, OUTPUT);
    pinMode(SHOWER_1_BUTTON, INPUT_PULLUP);
    pinMode(SHOWER_2_BUTTON, INPUT_PULLUP);
    // INPUT_PULLUP LOW = натиснуто, HIGH = відпущено
    lastShower1State = (digitalRead(SHOWER_1_BUTTON) == LOW);
    lastShower2State = (digitalRead(SHOWER_2_BUTTON) == LOW);
    lastQueueButtonState = (digitalRead(QUEUE_BUTTON) == LOW);
    lastQueueButtonPress = 0; // Ініціалізація для затримки

    // Ініціалізація масиву debounce для кнопок температури
    for (int r = 0; r < 2; r++)
    {
        for (int c = 0; c < 4; c++)
        {
            lastTemperatureButtonPress[r][c] = 0;
        }
    }
};

void SmartShower::updateTemperatureButtons()
{
    ulong currentTime = millis();
    for (int r = 0; r < 2; r++)
    {
        digitalWrite(temperatureGrounds[r], LOW);
        delay(5); // Затримка для стабільності

        for (int c = 0; c < 4; c++)
        {
            if (digitalRead(temperatureButtons[c]) == LOW)
            {
                // Debounce - перевірка, чи пройшло достатньо часу з останнього натискання
                if (currentTime - lastTemperatureButtonPress[r][c] >= 500)
                {
                    if (r == 0 && shower1.getWhoNow() != "")
                    {
                        shower1.setWaterTemperature(c + 1);
                        buzzerBeep(false); // Успіх
                        lastTemperatureButtonPress[r][c] = currentTime;
                    }
                    if (r == 1 && shower2.getWhoNow() != "")
                    {
                        shower2.setWaterTemperature(c + 1);
                        buzzerBeep(false); // Успіх
                        lastTemperatureButtonPress[r][c] = currentTime;
                    }
                }
            }
        }

        digitalWrite(temperatureGrounds[r], HIGH);
    }
}

bool SmartShower::addingToQueue(const String &id)
{
    // Перевірка робочого часу
    if (!isWorkingTime())
    {
        return false; // Не робочий час
    }

    if (!queue.isFull())
    {
        queue.push(id);
        return true;
    }
    return false;
}

void SmartShower::queueReductionByIndex(const int8_t index)
{
    int size = queue.size();
    if (index < 0 || index >= size)
        return;

    String temp[size];

    for (int i = 0; i < size; i++)
    {
        temp[i] = queue.shift();
    }

    for (int i = 0; i < size; i++)
    {
        if (i != index)
        {
            queue.push(temp[i]);
        }
    }
}
void SmartShower::queueReduction(const String &id)
{
    queueReductionByIndex(isInQueue(id));
}

void SmartShower::queueDisplay()
{
    oled.clear();
    oled.setCursor(0, 0);
    oled.print("Черга: ");
    oled.print(queue.size());
    oled.setCursor(0, 1);
    if (queue.isEmpty())
    {
        oled.print("Empty");
    }
    else
    {
        oled.print("Наступний: ");
        if (queue.first() == "0")
        {
            oled.print("(Зареєстровано кнопкою)");
        }
        else
        {
            oled.print(queue.first());
        }
    }
    oled.update();
}

String SmartShower::getFirstId()
{
    if (!queue.isEmpty())
        return queue.first();
    return "-1";
}

void SmartShower::pressQueueButton()
{
    // Перевірка робочого часу
    if (!isWorkingTime())
    {
        buzzerBeep(true); // Помилка - не робочий час
        showErrorOnOled("Не робочий час!");
        return;
    }

    if (!queue.isFull())
    {
        queue.push("0");
        buzzerBeep(false); // Успіх
    }
    else
    {
        buzzerBeep(true); // Помилка - черга заповнена
        showErrorOnOled("Черга заповнена!");
    }
}

int8_t SmartShower::isInQueue(const String &id)
{
    for (u8_t i = 0; i < queue.size(); i++)
    {
        if (queue[i] == id)
            return i;
    }
    return -1;
}

void SmartShower::buzzerBeep(bool isError)
{
    if (isError)
    {
        // Помилка - низький тон, довший сигнал
        tone(BUZZER, 300, 500);
    }
    else
    {
        // Підтвердження - високий тон, короткий сигнал
        tone(BUZZER, 800, 200);
    }
}

void SmartShower::showErrorOnOled(const String &error)
{
    errorMessage = error;
    errorDisplayStart = millis();
}

void SmartShower::showNonWorkingTimeAnimation()
{
    // TODO: Додати анімацію для не робочого часу
    if (millis() - lastAnimationUpdate < 100)
        return;
    lastAnimationUpdate = millis();

    oled.clear();
    oled.setScale(2);
    oled.setCursor(animationOffset, 0);
    oled.print("НЕ РОБОЧИЙ");
    oled.setCursor(animationOffset, 2);
    oled.print("ЧАС");
    oled.update();

    animationOffset -= 2;
    if (animationOffset < -100)
        animationOffset = 128;
}

bool SmartShower::isWorkingTime()
{
    timeClient.update();
    u8_t hour = static_cast<u8_t>(timeClient.getHours());
    if (hour >= NIGHT_TIME_START && hour < NIGHT_TIME_FINISH)
    {
        return false;
    }
    if (hour >= MIDDAY_TIME_START && hour < MIDDAY_TIME_FINISH)
    {
        return false;
    }
    return true;
}

void SmartShower::clearQueueIfNonWorkingTime()
{
    if (!isWorkingTime() && !queue.isEmpty())
    {
        while (!queue.isEmpty())
        {
            queue.shift();
        }
    }
}

void SmartShower::handleShowerButton(Shower &shower, bool currentState, bool &lastState)
{
    // currentState це результат digitalRead (LOW=0/false, HIGH=1/true)
    bool buttonPressed = (currentState == LOW); // LOW = натиснуто

    if (buttonPressed && !lastState)
    {
        // Кнопка натиснута - душ зайнятий
        // Перевірка робочого часу
        if (!isWorkingTime())
        {
            buzzerBeep(true); // Помилка - не робочий час
            showErrorOnOled("Не робочий час!");
            return;
        }

        if (!queue.isEmpty())
        {
            String userId = queue.shift();
            shower.setWhoNow(userId);
            buzzerBeep(false); // Успіх
            queueReductionMessage(GROUP_ID);
        }
        else
        {
            // Черга порожня, але душ зайнятий
            shower.setWhoNow("0");
            buzzerBeep(false); // Успіх
        }
    }
    else if (!buttonPressed && lastState)
    {
        // Кнопка відпущена - душ вільний
        // whoNow залишається, щоб можна було встановити температуру
        buzzerBeep(false); // Успіх
    }

    lastState = buttonPressed;
    shower.updateDisplay();
}

void SmartShower::handleQueueButton()
{
    bool currentState = digitalRead(QUEUE_BUTTON);
    bool buttonPressed = (currentState == LOW); // Для INPUT_PULLUP LOW = натиснуто

    if (buttonPressed && !lastQueueButtonState)
    {
        // Перевірка затримки 3 секунди
        ulong currentTime = millis();
        if (lastQueueButtonPress == 0 || currentTime - lastQueueButtonPress >= 3000)
        {
            pressQueueButton();
            lastQueueButtonPress = currentTime;
        }
        else
        {
            // Затримка не пройшла
            buzzerBeep(true); // Помилка
            showErrorOnOled("Зачекайте 3 сек!");
        }
    }

    lastQueueButtonState = buttonPressed;
}

void SmartShower::run()
{
    timeClient.update();

    if (!isWorkingTime())
    {
        clearQueueIfNonWorkingTime();
        showNonWorkingTimeAnimation();
        shower1.updateDisplay();
        shower2.updateDisplay();
        return;
    }

    // Обробка кнопок душів
    bool shower1ButtonState = digitalRead(SHOWER_1_BUTTON);
    bool shower2ButtonState = digitalRead(SHOWER_2_BUTTON);

    handleShowerButton(shower1, shower1ButtonState, lastShower1State);
    handleShowerButton(shower2, shower2ButtonState, lastShower2State);

    // Обробка кнопки черги
    handleQueueButton();

    // Обробка кнопок температури
    updateTemperatureButtons();

    // Показ помилки на OLED (якщо є)
    if (errorMessage != "" && millis() - errorDisplayStart < 5000)
    {
        oled.clear();
        oled.setCursor(0, 0);
        oled.print("Помилка:");
        oled.setCursor(0, 1);
        oled.print(errorMessage);
        oled.update();
    }
    else if (errorMessage != "" && millis() - errorDisplayStart >= 5000)
    {
        errorMessage = ""; // Очищаємо помилку після 5 секунд
    }

    // Оновлення OLED дисплею (раз на секунду)
    if (millis() - lastOledUpdate > 1000 && errorMessage == "")
    {
        queueDisplay();
        lastOledUpdate = millis();
    }
}