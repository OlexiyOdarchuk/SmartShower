#pragma once
#include <Arduino.h>
#include <FastBot2.h>
#include <GyverOLED.h>
#include <CircularBuffer.hpp>
#include <Ticker.h>
#include <Preferences.h>
#include <freertos/FreeRTOS.h>
#include <freertos/semphr.h>
#include <Shower.hpp>

struct QueueEntry
{
    String id;
    String name;

    String displayName() const
    {
        if (id == "0") return "Кнопка";
        if (name.length() > 0) return name;
        return id;
    }
};

// Атомарний знімок першого елемента черги.
struct QueueHead
{
    String id;
    String displayName;
    bool isEmpty;
};

// Результат спроби додатися в чергу.
struct JoinResult
{
    enum Status { ADDED, ALREADY_IN, FULL, OFF_HOURS } status;
    int position; // 1-based, актуально тільки для ADDED і ALREADY_IN
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
    JoinResult tryJoin(const String &id, const String &name);
    bool       leaveQueue(const String &id, bool &wasFirstOut);
    int8_t     isInQueue(const String &id);
    uint8_t    snapshotQueue(QueueEntry *out, uint8_t maxOut);
    QueueHead  getHead();
    uint8_t    queueLen();
    void       clearQueue();
    bool       isWorkingTime();

    // Прапор для бот-задачі: «настав час сповістити наступного».
    volatile bool pendingNotifyNext = false;

    // Прохання пікнути з боку бот-задачі.
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

    // — NVS —
    Preferences prefs;

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

    // — авто-вибуття першого з черги —
    String firstNotifyId;
    ulong  firstNotifyAt = 0;

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
    void persistQueueUnlocked();
    void restoreQueueOnBoot();
    void runAutoRemoveTick();
    void armFirstNotify(const QueueHead &head);
};
