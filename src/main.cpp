#include <Arduino.h>
#include <secrets.h>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <NTPClient.h>
#include <bot.h>
#include <Shower.h>

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
    bot.setToken(BOT_TOKEN);
    bot.setPollMode(fb::Poll::Long, 60000);

    // Виключити всі оновлення
    bot.updates.clearAll();

    // Включаю тільки потрібні
    bot.updates.set(fb::Updates::Type::Message);
}
void loop()
{
    bot.tick();
    bot.attachUpdate(updateh);
    digitalWrite(13, LOW);
    delay(1000);
    digitalWrite(13, HIGH);
    delay(1000);
}