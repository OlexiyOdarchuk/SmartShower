#include <Arduino.h>
#include <secrets.h>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.h>

void updateh(fb::Update &u)
{
    Serial.println("New message!");

    if (u.message().text() == "/start")
    {
        fb::Message msg("Hello!", u.message().chat().id());
        bot.sendMessage(msg, true);
    }
}
