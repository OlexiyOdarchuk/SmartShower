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
    bool isWorkingTime();

private:
    CircularBuffer<String, 30> queue;
    FastBot2 &bot;
    Shower &shower1;
    Shower &shower2;
    GyverOLED<SSD1306_128x32, OLED_NO_BUFFER> &oled;
    u8_t temperatureGrounds[2];
    const u8_t temperatureButtons[4];
    bool lastShower1State = false;
    bool lastShower2State = false;
    bool lastQueueButtonState = false;
    ulong lastOledUpdate = 0;
    ulong lastAnimationUpdate = 0;
    ulong errorDisplayStart = 0;
    ulong lastQueueButtonPress = 0;
    ulong lastTemperatureButtonPress[2][4] = {{0}}; // Debounce для кнопок температури
    String errorMessage = "";
    int animationOffset = 0;
    void queueDisplay();
    void pressQueueButton();
    void updateTemperatureButtons();
    void queueReductionByIndex(const int8_t index);
    void handleShowerButton(Shower &shower, bool currentState, bool &lastState);
    void handleQueueButton();
    void buzzerBeep(bool isError = false);
    void showErrorOnOled(const String &error);
    void showNonWorkingTimeAnimation();
    void clearQueueIfNonWorkingTime();
};