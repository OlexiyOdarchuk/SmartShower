#include <Arduino.h>
#include <secrets.hpp>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <logic.hpp>
#include <GyverOLED.h>
#include <SmartShower.hpp>

Shower shower1(1, SHOWER_1_TIMER_DIO, SHOWER_1_TIMER_CLK, SHOWER_1_BUTTON,
               SHOWER_1_RED_LED, SHOWER_1_GREEN_LED);
Shower shower2(2, SHOWER_2_TIMER_DIO, SHOWER_2_TIMER_CLK, SHOWER_2_BUTTON,
               SHOWER_2_RED_LED, SHOWER_2_GREEN_LED);
GyverOLED<SSD1306_128x32, OLED_NO_BUFFER> oled;
SmartShower smartShower(bot, shower1, shower2, oled);
