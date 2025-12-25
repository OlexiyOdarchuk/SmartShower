#pragma once
#include <Arduino.h>
#include <CircularBuffer.hpp>
#include <GyverSegment.h>

struct WaterTemperature
{
    uint8_t temperature;
    String user;
    String time;

    String getInfo();
};

class Shower
{
public:
    Shower(u8_t displayDIO, u8_t displayCLK, u8_t button, u8_t temperatureGround, u8_t redLed, u8_t greenLed);

    String getWaterTemperature();
    void updateDisplay();
    u8_t getTemperatureGround();
    void setWaterTemperature(const u8_t temperature);
    void setWhoNow(const String &id);
    void ledControl();
    bool isBusyNow();
    String getWhoNow();

private:
    WaterTemperature waterTemperature;
    const u8_t button;
    const u8_t redLed;
    const u8_t greenLed;
    const u8_t temperatureGround;
    ulong start = 0;
    bool getChange();
    bool isBusy = false;
    bool isChange = false;
    Disp1637Colon showerTimer;
    String whoNow;
};