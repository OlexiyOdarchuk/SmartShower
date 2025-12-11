#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <Shower.hpp>
#include <SmartShower.hpp>
#include <CircularBuffer.hpp>

SmartShower::SmartShower(FastBot2 &bot, Shower &shower1, Shower &shower2, u8_t temperatureButton1, u8_t temperatureButton2, u8_t temperatureButton3, u8_t temperatureButton4) : bot(bot), shower1(shower1), shower2(shower2), temperatureButtons({temperatureButton1, temperatureButton2, temperatureButton3, temperatureButton4})
{
    temperatureGrounds[0] = shower1.getTemperatureGround();
    temperatureGrounds[1] = shower2.getTemperatureGround();

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
                    shower1.setWaterTemperature(queue);
                }
                if (r == 1)
                {
                    shower2.setWaterTemperature(queue);
                }
            }
        }

        digitalWrite(temperatureGrounds[r], HIGH);
    }
}

void SmartShower::addingToQueue(String &id)
{
    if (!queue.isFull())
    {
        queue.push(id);
    }
}

void SmartShower::queueReduction(Shower &shower)
{
    if (!queue.isEmpty()) // ! Винести це перед викликом!!!
    {
        String element = queue.shift();
        if (queue.first() != "0")
        {
            fb::Message msg;
            msg.text = "" + queue.first();
            bot.sendMessage(msg);
            // TODO: Перенести на форматування і написати нормальний текст
        }
    }
}

void SmartShower::pressShowerButton(Shower &shower)
{
    queueReduction(shower);
}