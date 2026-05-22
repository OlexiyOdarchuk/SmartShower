#pragma once
#include <Arduino.h>
#include <GyverSegment.h>

struct WaterTemperature
{
    uint8_t temperature = 0;
    String user;
    String time;

    String getInfo() const;
};

class Shower
{
public:
    Shower(uint8_t id,
           uint8_t displayDIO, uint8_t displayCLK,
           uint8_t button,
           uint8_t redLed, uint8_t greenLed);

    void init();
    void updateDisplay();
    void setWaterTemperature(uint8_t temperature);
    String getWaterTemperature() const;
    void setWhoNow(const String &id);
    String getWhoNow() const;
    void clearWhoNow();
    bool isBusyNow() const;

private:
    const uint8_t id;
    const uint8_t button;
    const uint8_t redLed;
    const uint8_t greenLed;
    Disp1637Colon showerTimer;
    WaterTemperature waterTemperature;
    String whoNow;
    ulong timerStart = 0;
    bool isBusy = false;
    char lastDisplayBuf[6] = "";

    bool pollButton();
    void applyLed() const;
};
