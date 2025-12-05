#pragma once
#include <Arduino.h>
#include <secrets.h>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.h>

struct ShowerWaterTemperature
{
    uint8_t temperature;
    String user;
    String time; // getFormattedTime вертає стрінг, але думаю з тим ще буде час побавитися
};

class Shower
{
public:
    Shower(u8_t displayDIO, u8_t displayCLK, u8_t button, u8_t temperatureButton1, u8_t temperatureButton2, u8_t temperatureButton3, u8_t temperatureButton4);

private:
    ShowerWaterTemperature showerWaterTemperature;
    u8_t button;
    u8_t temperatureButtons[4];
    ulong start = 0;
    bool isRunning = false;
    Disp1637Colon showerTimer;

    void updateDisplay();
};