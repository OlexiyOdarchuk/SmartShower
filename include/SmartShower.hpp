#pragma once
#include <Arduino.h>
#include <FastBot2.h>
#include <GyverOLED.h>
#include <CircularBuffer.hpp>
#include <Shower.hpp>

class SmartShower
{
public:
    SmartShower(FastBot2 &botRef, Shower &shower1Ref, Shower &shower2Ref, GyverOLED<SSD1306_128x32, OLED_NO_BUFFER> &oledRef, u8_t temperatureButton1, u8_t temperatureButton2, u8_t temperatureButton3, u8_t temperatureButton4);
    void run();
    String getFirstId();
    void queueReduction(const String &id);
    int8_t isInQueue(const String &id);
    bool addingToQueue(const String &id);

private:
    CircularBuffer<String, 30> queue;
    FastBot2 &bot;
    Shower &shower1;
    Shower &shower2;
    GyverOLED<SSD1306_128x32, OLED_NO_BUFFER> &oled;
    u8_t temperatureGrounds[2];
    const u8_t temperatureButtons[4];
    void queueDisplay();
    void pressShowerButton(Shower &shower);
    void pressQueueButton();
    void updateTemperatureButtons();
    void queueReductionByIndex(const int8_t index);
};