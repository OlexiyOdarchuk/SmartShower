#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <NTPClient.h>
#include <bot.hpp>
#include <Shower.hpp>
#include <logic.hpp>

// Підключення душів
// Shower shower1(SHOWER_1_TIMER_DIO, SHOWER_1_TIMER_CLK, SHOWER_1_BUTTON, SHOWER_1_TEMPERATURE_BUTTON_1, SHOWER_1_TEMPERATURE_BUTTON_2, SHOWER_1_TEMPERATURE_BUTTON_3, SHOWER_1_TEMPERATURE_BUTTON_4);
// Shower shower2(SHOWER_2_TIMER_DIO, SHOWER_2_TIMER_CLK, SHOWER_2_BUTTON, SHOWER_2_TEMPERATURE_BUTTON_1, SHOWER_2_TEMPERATURE_BUTTON_2, SHOWER_2_TEMPERATURE_BUTTON_3, SHOWER_2_TEMPERATURE_BUTTON_4);

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

    // Налаштування дисплеїв
    pinMode(SHOWER_1_TIMER_DIO, OUTPUT);
    pinMode(SHOWER_1_TIMER_CLK, OUTPUT);
    pinMode(SHOWER_1_BUTTON, INPUT_PULLUP);
}
void loop()
{
    bot.tick();
}