#pragma once
#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <CircularBuffer.hpp>
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
    Shower(u8_t displayDIO, u8_t displayCLK, u8_t button, u8_t temperatureGround, u8_t redLed, u8_t greenLed);

    String getWaterTemperature();
    void updateDisplay();
    void getTemperatureButtons(u8_t buttons[4]);
    u8_t getTemperatureGround();
    void setWaterTemperature(CircularBuffer<String, 30> &queue);

private:
    WaterTemperature waterTemperature;
    u8_t button;
    u8_t redLed;
    u8_t greenLed;
    u8_t temperatureGround;
    ulong start = 0;
    bool isRunning = false;
    Disp1637Colon showerTimer;
};