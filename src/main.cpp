#include <Arduino.h>
#include <secrets.h>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <NTPClient.h>
#include <bot.h>
#include <Shower.h>

// Підключення душів
Shower shower1(SHOWER_1_TIMER_DIO, SHOWER_1_TIMER_CLK, SHOWER_1_BUTTON, SHOWER_1_TEMPERATURE_BUTTON_1, SHOWER_1_TEMPERATURE_BUTTON_2, SHOWER_1_TEMPERATURE_BUTTON_3, SHOWER_1_TEMPERATURE_BUTTON_4);
Shower shower2(SHOWER_2_TIMER_DIO, SHOWER_2_TIMER_CLK, SHOWER_2_BUTTON, SHOWER_2_TEMPERATURE_BUTTON_1, SHOWER_2_TEMPERATURE_BUTTON_2, SHOWER_2_TEMPERATURE_BUTTON_3, SHOWER_2_TEMPERATURE_BUTTON_4);

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

    // Налаштування бота
    bot.setPollMode(fb::Poll::Long, 60000);

    // Виключити всі оновлення
    bot.updates.clearAll();

    // Включаю тільки потрібні
    bot.updates.set(fb::Updates::Type::Message);
}
void loop()
{
    shower1.run();
    shower2.run();
    bot.tick();
    bot.attachUpdate(updateh);
    digitalWrite(13, LOW);
    delay(1000);
    digitalWrite(13, HIGH);
    delay(1000);
}
