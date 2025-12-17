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
};

void SmartShower::updateTemperatureButtons()
{
    for (int r = 0; r < 2; r++)
    {

        digitalWrite(temperatureGrounds[r], LOW);

        for (int c = 0; c < 4; c++)
        {
            if (digitalRead(temperatureButtons[c]) == LOW)
            {
                if (r == 0)
                {
                    shower1.setWaterTemperature(queue, c + 1);
                }
                if (r == 1)
                {
                    shower2.setWaterTemperature(queue, c + 1);
                }
            }
        }

        digitalWrite(temperatureGrounds[r], HIGH);
    }
}

bool SmartShower::addingToQueue(const String &id)
{
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

void SmartShower::pressShowerButton(Shower &shower)
{
    if (!queue.isEmpty()) // ! Винести це перед викликом!!!
    {
        shower.setWhoNow(queue.shift());
        queueReductionMessage(GROUP_ID);
    }
}

void SmartShower::queueDisplay()
{
    // TODO: написати обробку для дій oled
}

String SmartShower::getFirstId()
{
    if (!queue.isEmpty())
        return queue.first();
    return "-1";
}

void SmartShower::pressQueueButton()
{
    if (!queue.isFull())
    {
        queue.push("0");
    }
    // TODO: Отут бузер ще має бути
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

void SmartShower::run()
{
    updateTemperatureButtons();
    // TODO: Написати фукнцію для роботи, яка вже буде обробляти всі натискання
}