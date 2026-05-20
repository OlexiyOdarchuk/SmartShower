#pragma once
#include <Arduino.h>
#include <FastBot2.h>
#include <GyverOLED.h>
#include <CircularBuffer.hpp>
#include <Ticker.h>
#include <freertos/FreeRTOS.h>
#include <freertos/semphr.h>
#include <Shower.hpp>

struct QueueEntry
{
    String id;
    String name; // username / first_name / "" для "Кнопка"

    String displayName() const
    {
        if (id == "0") return "Кнопка";
        if (name.length() > 0) return name;
        return id;
    }
};

// Атомарний знімок першого елемента черги — щоб бот не робив два окремі locked-виклики
struct QueueHead
{
    String id;
    String displayName;
    bool isEmpty;
};

struct TempButtonState
{
    bool raw       = false;
    bool stable    = false;
    ulong changeAt = 0;
    ulong firedAt  = 0;
};

class SmartShower
{
public:
    SmartShower(FastBot2 &botRef,
                Shower &shower1Ref, Shower &shower2Ref,
                GyverOLED<SSD1306_128x32, OLED_NO_BUFFER> &oledRef);

    void init();
    void run();

    // — інтерфейс для бота —
    bool      addingToQueue(const String &id, const String &name = "");
    void      queueReduction(const String &id);
    int8_t    isInQueue(const String &id);
    QueueEntry getQueueAt(uint8_t index);
    QueueHead  getHead();              // атомарний знімок першого
    uint8_t   queueLen();
    void      clearQueue();
    bool      isWorkingTime();          // кешоване значення

    // Прапор для бот-задачі: «настав час сповістити наступного».
    volatile bool pendingNotifyNext = false;

    // Прохання пікнути з боку бот-задачі. Виконається на core 1 у run().
    void requestBeep(bool isError);

private:
    // — залежності —
    FastBot2 &bot;
    Shower &shower1;
    Shower &shower2;
    GyverOLED<SSD1306_128x32, OLED_NO_BUFFER> &oled;

    // — стан черги —
    CircularBuffer<QueueEntry, 30> queue;
    SemaphoreHandle_t queueMutex = nullptr;

    // — кеш working-time —
    bool  workingTimeCached = true;
    ulong workingTimeUpdatedAt = 0;

    // — стан кнопок —
    bool lastShower1ButtonState = false;
    bool lastShower2ButtonState = false;
    bool lastQueueButtonState   = false;
    bool lastShower1Busy        = false;
    bool lastShower2Busy        = false;
    ulong lastQueueButtonPress  = 0;
    TempButtonState tempBtn[2][4];

    // — стан OLED —
    String lastOledRender;
    ulong  lastOledUpdate      = 0;
    ulong  lastAnimationUpdate = 0;
    ulong  errorDisplayStart   = 0;
    String errorMessage;
    int    animationOffset     = 0;

    // — buzzer —
    Ticker buzzerOffTicker;
    volatile bool pendingBeep = false;
    volatile bool pendingBeepIsError = false;

    // — приватні методи —
    void refreshWorkingTime();
    bool computeWorkingTime();
    void handleShowerButton(Shower &shower, bool &lastState, int buttonPin);
    void handleQueueButton();
    void handleTempButton(uint8_t s, uint8_t c, int pin, Shower &shower);
    void updateTemperatureButtons();
    void pressQueueButton();
    void detectShowerRelease();
    void queueReductionByIndexUnlocked(int8_t index);
    void buzzerBeep(bool isError = false);
    void processPendingBeep();
    void renderOled(const String &content);
    void queueDisplay();
    void showErrorOnOled(const String &error);
    void showNonWorkingTimeAnimation();
    void clearQueueIfNonWorkingTime();
};
