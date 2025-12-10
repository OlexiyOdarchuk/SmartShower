#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <logic.hpp>

FastBot2 bot(BOT_TOKEN);

void updateh(fb::Update &u)
{
    if (u.message().chat().id() == GROUP_ID)
    {
        u8_t hour = static_cast<u8_t>(timeClient.getHours());
        if ((hour > NIGHT_TIME_START && hour < NIGHT_TIME_FINISH) || (hour > MIDDAY_TIME_START && hour < MIDDAY_TIME_FINISH))
        {
            Serial.println("New message!");

            if (u.message().text() == "/start")
            {
                fb::Message msg("Hello!", u.message().chat().id());
                bot.sendMessage(msg, true);
            }
        }
    }
}

void getInfo()
{
    String wt1 = shower1.getWaterTemperature();
    String wt2 = shower2.getWaterTemperature();
    fb::Message
    msg();
}