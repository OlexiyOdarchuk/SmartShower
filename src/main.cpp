#include <Arduino.h>
#include <secrets.h>
#include <TM1637Display.h>
#include <FastBot2.h>
#include <WiFi.h>

FastBot2 bot(BOT_TOKEN);

void updateh(fb::Update &u)
{
    Serial.println("New message!");

    if (u.message().text() == "/start")
    {
        fb::Message msg;
        msg.chatID = u.message().chat().id();
        msg.text = "Hello";
        bot.sendMessage(msg, false);
    }
}

void setup()
{
    Serial.begin(115200);
    WiFi.begin(WIFI_SSID, WIFI_PASS);
    while (WiFi.status() != WL_CONNECTED)
    {
        delay(500);
        Serial.print(".");
    }
    Serial.println("Connected");
    Serial.println(WiFi.localIP());
    bot.setPollMode(fb::Poll::Long, 60000);
}
void loop()
{
    bot.attachUpdate(updateh);
    bot.tick();
}