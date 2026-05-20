#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <NTPClient.h>
#include <bot.hpp>
#include <Shower.hpp>
#include <SmartShower.hpp>
#include <logic.hpp>

static TaskHandle_t botTaskHandle = nullptr;

static void botTaskFn(void *)
{
    for (;;)
    {
        bot.tick();
        // Якщо фізична кнопка прийняла наступного з черги, треба сповістити нового першого.
        if (smartShower.pendingNotifyNext)
        {
            smartShower.pendingNotifyNext = false;
            notifyNextInQueue(GROUP_ID);
        }
        vTaskDelay(pdMS_TO_TICKS(5));
    }
}

void setup()
{
    Serial.begin(115200);
    smartShower.init();

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
    bot.updates.clearAll();
    bot.updates.set(fb::Updates::Type::Message | fb::Updates::Type::CallbackQuery);
    bot.attachUpdate(updateh);

    // Бот живе на ядрі 0 (PRO_CPU), щоб блокуючі HTTP-запити не зачіпали
    // фізичну логіку, яка виконується на ядрі 1 (loop()).
    xTaskCreatePinnedToCore(
        botTaskFn,
        "botTask",
        16384,         // стек: TLS handshake + JSON-парсинг, потрібно багато
        nullptr,
        1,             // пріоритет
        &botTaskHandle,
        0              // PRO_CPU
    );
    Serial.println("[main] Bot task pinned to core 0");
}

void loop()
{
    smartShower.run();
    delay(1); // віддати трохи часу планувальнику
}
