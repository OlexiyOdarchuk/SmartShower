#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <NTPClient.h>
#include <bot.hpp>
#include <Shower.hpp>
#include <logic.hpp>

void setup()
{
    Serial.begin(115200);
    // Підключення до інтернету
    WiFi.begin(WIFI_SSID, WIFI_PASS);
    while (WiFi.status() != WL_CONNECTED)
    {
        delay(500);
        Serial.print(".");
    }
    Serial.println("Connected");
    Serial.println(WiFi.localIP());

    // Налаштування часу
    timeClient.begin();

    // Налаштування бота
    bot.setPollMode(fb::Poll::Long, 60000);

    // Виключити всі оновлення
    bot.updates.clearAll();

    // Включаю тільки потрібні
    bot.updates.set(fb::Updates::Type::Message | fb::Updates::Type::CallbackQuery);

    // Attach update handler
    bot.attachUpdate(updateh);
}
void loop()
{
    bot.tick();
}