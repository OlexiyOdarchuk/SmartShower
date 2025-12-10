#pragma once

// Мережеві налаштування
#define WIFI_SSID "MyWifi"
#define WIFI_PASS "mySuperPuperParol"
#define BOT_TOKEN "0123456789:BOTShowERToNKen_CLI"
#define ADMIN_ID "1433760480"
#define GROUP_ID "-100161234841"

// Душ 1
#define SHOWER_1_TIMER_DIO 1
#define SHOWER_1_TIMER_CLK 2
#define SHOWER_1_BUTTON 3
#define SHOWER_1_GROUND 4
#define SHOWER_1_GREEN_LED 5
#define SHOWER_1_RED_LED 6

// Душ 2
#define SHOWER_2_TIMER_DIO 7
#define SHOWER_2_TIMER_CLK 8
#define SHOWER_2_BUTTON 9
#define SHOWER_2_GROUND 10
#define SHOWER_2_GREEN_LED 11
#define SHOWER_2_RED_LED 12

// Кнопки для клавіатури (Вони зроблені як матриця, тобто 4 спільні піни і 2 різні землі)
#define SHOWER_TEMPERATURE_BUTTON_1 13
#define SHOWER_TEMPERATURE_BUTTON_2 14
#define SHOWER_TEMPERATURE_BUTTON_3 15
#define SHOWER_TEMPERATURE_BUTTON_4 16

// Загальні піни
#define QUEUE_BUTTON 17
#define BLACK_BUTTON 18
#define BUZZER 19
#define OLED_SCK 20
#define OLED_SDA 21

// Робочий час
#define NIGHT_TIME_START 0
#define NIGHT_TIME_FINISH 6
#define MIDDAY_TIME_START 11
#define MIDDAY_TIME_FINISH 14