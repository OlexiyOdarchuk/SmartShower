#pragma once
#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>

struct WaterTemperature
{
    uint8_t temperature;
    String user;
    String time; // getFormattedTime вертає стрінг, але думаю з тим ще буде час побавитися
    // ulong time; // Можна записувати його просто як millis() і тоді буде видно скільки часу назад було оцінено

    String getInfo();
};

class Shower
{
public:
    Shower(u8_t displayDIO, u8_t displayCLK, u8_t button, u8_t temperatureGround);
    Shower(u8_t displayDIO, u8_t displayCLK, u8_t button, u8_t temperatureGround, u8_t temperatureButton1, u8_t temperatureButton2, u8_t temperatureButton3, u8_t temperatureButton4);

    String getWaterTemperature();
    void updateDisplay();

private:
    WaterTemperature waterTemperature;
    u8_t button;
    u8_t temperatureButtons[4] = {SHOWER_TEMPERATURE_BUTTON_1, SHOWER_TEMPERATURE_BUTTON_2, SHOWER_TEMPERATURE_BUTTON_3, SHOWER_TEMPERATURE_BUTTON_4};
    u8_t temperatureGround;
    ulong start = 0;
    bool isRunning = false;
    Disp1637Colon showerTimer;

    void setWaterTemperature();
};