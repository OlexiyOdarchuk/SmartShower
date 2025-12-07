#include <Arduino.h>
#include <secrets.h>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.h>
#include <logic.h>

FastBot2 bot(BOT_TOKEN);

void updateh(fb::Update &u)
{
    Serial.println("New message!");

    if (u.message().text() == "/start")
    {
        fb::Message msg("Hello!", u.message().chat().id());
        bot.sendMessage(msg, true);
    }
}

void getInfo()
{
    String wt1 = shower1.getWaterTemperature();
    String wt2 = shower2.getWaterTemperature();
    fb::Message
    msg();
}