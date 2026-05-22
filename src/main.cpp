#include <Arduino.h>
#include <secrets.hpp>
#include <FastBot2.h>
#include <WiFi.h>
#include <esp_task_wdt.h>
#include <bot.hpp>
#include <Shower.hpp>
#include <SmartShower.hpp>
#include <logic.hpp>
#include <timeutil.hpp>

namespace {
constexpr uint32_t WIFI_TIMEOUT_MS = 30000;
constexpr uint32_t WDT_TIMEOUT_S   = 15;
} // namespace

static TaskHandle_t botTaskHandle = nullptr;

static void botTaskFn(void *)
{
    for (;;)
    {
        bot.tick();
        if (smartShower.pendingNotifyNext)
        {
            smartShower.pendingNotifyNext = false;
            notifyNextInQueue(GROUP_ID);
        }
        vTaskDelay(pdMS_TO_TICKS(5));
    }
}

static void connectWiFi()
{
    WiFi.mode(WIFI_STA);
    WiFi.setAutoReconnect(true);
    WiFi.begin(WIFI_SSID, WIFI_PASS);
    ulong started = millis();
    while (WiFi.status() != WL_CONNECTED)
    {
        if (millis() - started > WIFI_TIMEOUT_MS)
        {
            Serial.println("\n[WiFi] connect timeout, restarting...");
            delay(1000);
            ESP.restart();
        }
        delay(500);
        Serial.print(".");
    }
    Serial.printf("\n[WiFi] connected, IP=%s\n", WiFi.localIP().toString().c_str());
}

void setup()
{
    Serial.begin(115200);
    smartShower.init();

    connectWiFi();

    timeutil::setup();

    bot.setPollMode(fb::Poll::Long, 30000);
    bot.updates.clearAll();
    bot.updates.set(fb::Updates::Type::Message | fb::Updates::Type::CallbackQuery);
    bot.attachUpdate(updateh);

    // Бот живе на ядрі 0 (PRO_CPU), щоб блокуючі HTTP не зачіпали loop.
    xTaskCreatePinnedToCore(
        botTaskFn,
        "botTask",
        16384,
        nullptr,
        1,
        &botTaskHandle,
        0
    );
    Serial.println("[main] Bot task pinned to core 0");

    // Watchdog на основний loop — якщо щось зависне, плата перезавантажиться.
    esp_task_wdt_init(WDT_TIMEOUT_S, true);
    esp_task_wdt_add(nullptr);
    Serial.printf("[main] Task WDT armed: %lus\n", (unsigned long)WDT_TIMEOUT_S);
}

void loop()
{
    esp_task_wdt_reset();
    smartShower.run();
    delay(1);
}
