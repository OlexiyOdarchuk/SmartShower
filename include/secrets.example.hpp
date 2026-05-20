#pragma once

// Мережеві налаштування
#define WIFI_SSID "MyWifi"
#define WIFI_PASS "mySuperPuperParol"
#define BOT_TOKEN "0123456789:BOTShowERToNKen_CLI"
#define ADMIN_ID "1433760480"
#define GROUP_ID "-100161234841"

// Душ 1
#define SHOWER_1_TIMER_DIO 26
#define SHOWER_1_TIMER_CLK 14
#define SHOWER_1_BUTTON 23
#define SHOWER_1_TEMPERATURE_BUTTON_1 15
#define SHOWER_1_TEMPERATURE_BUTTON_2 2
#define SHOWER_1_TEMPERATURE_BUTTON_3 13
#define SHOWER_1_TEMPERATURE_BUTTON_4 12
#define SHOWER_1_GREEN_LED 17
#define SHOWER_1_RED_LED 19

// Душ 2
// УВАГА: піни 34, 35 на ESP32 — input-only без внутрішньої підтяжки.
// Потрібні зовнішні pull-up резистори (10k до 3.3V).
#define SHOWER_2_TIMER_DIO 27
#define SHOWER_2_TIMER_CLK 14
#define SHOWER_2_BUTTON 4
#define SHOWER_2_TEMPERATURE_BUTTON_1 35
#define SHOWER_2_TEMPERATURE_BUTTON_2 34
#define SHOWER_2_TEMPERATURE_BUTTON_3 33
#define SHOWER_2_TEMPERATURE_BUTTON_4 32
#define SHOWER_2_GREEN_LED 5
#define SHOWER_2_RED_LED 18

// Загальні піни
#define QUEUE_BUTTON 16
#define BUZZER 25
#define OLED_SCK 22
#define OLED_SDA 21

// Не робочий час (включно: finish — це остання неробоча година)
#define NIGHT_TIME_START 0
#define NIGHT_TIME_FINISH (6-1)
#define MIDDAY_TIME_START 11
#define MIDDAY_TIME_FINISH (14-1)
