#include <Arduino.h>
#include <secrets.hpp>
#include <Shower.hpp>
#include <timeutil.hpp>
#include <logic.hpp>

Shower::Shower(uint8_t id,
               uint8_t displayDIO, uint8_t displayCLK,
               uint8_t button,
               uint8_t redLed, uint8_t greenLed)
    : id(id),
      button(button),
      redLed(redLed),
      greenLed(greenLed),
      showerTimer(displayDIO, displayCLK)
{
}

void Shower::init()
{
    pinMode(button,   INPUT_PULLUP);
    pinMode(redLed,   OUTPUT);
    pinMode(greenLed, OUTPUT);

    isBusy = (digitalRead(button) == LOW);
    applyLed();

    showerTimer.brightness(7);
    showerTimer.clear();
    showerTimer.print("FREE");
    showerTimer.update();
    lastDisplayBuf[0] = 'F'; lastDisplayBuf[1] = 'R'; lastDisplayBuf[2] = 'E';
    lastDisplayBuf[3] = 'E'; lastDisplayBuf[4] = 0;
    Serial.printf("[Shower %u] init OK, busy=%d\n", id, isBusy);
}

void Shower::applyLed() const
{
    digitalWrite(redLed,   isBusy ? HIGH : LOW);
    digitalWrite(greenLed, isBusy ? LOW  : HIGH);
}

bool Shower::pollButton()
{
    bool nowBusy = (digitalRead(button) == LOW);
    if (nowBusy == isBusy) return false;
    isBusy = nowBusy;
    Serial.printf("[Shower %u] state -> %s\n", id, isBusy ? "BUSY" : "FREE");
    applyLed();
    return true;
}

void Shower::updateDisplay()
{
    bool justChanged = pollButton();

    if (!isBusy)
    {
        if (justChanged || strcmp(lastDisplayBuf, "FREE") != 0)
        {
            showerTimer.setCursor(0);
            showerTimer.print("FREE");
            showerTimer.update();
            strncpy(lastDisplayBuf, "FREE", sizeof(lastDisplayBuf));
        }
        return;
    }

    if (justChanged)
    {
        timerStart = millis();
        Serial.printf("[Shower %u] timer started\n", id);
    }

    ulong elapsed = millis() - timerStart;
    uint8_t minutes = (elapsed / 60000UL) % 60;
    uint8_t seconds = (elapsed / 1000UL) % 60;

    char buf[6];
    snprintf(buf, sizeof(buf), "%02u%02u", minutes, seconds);

    // Оновлюємо TM1637 (bit-bang, ~10 мс) тільки коли текст справді змінився.
    if (strcmp(buf, lastDisplayBuf) != 0)
    {
        showerTimer.setCursor(0);
        showerTimer.print(buf);
        showerTimer.colon(true);
        showerTimer.update();
        strncpy(lastDisplayBuf, buf, sizeof(lastDisplayBuf));
    }
}

String WaterTemperature::getInfo() const
{
    if (temperature == 0) return "Температура не встановлена";
    String info;
    info.reserve(64);
    info  = "Температура: " + String(temperature) + "\n";
    info += "Час: " + time + "\n";
    info += (user == "0" || user.length() == 0)
            ? "Користувач: Зареєстровано кнопкою"
            : "Користувач: " + user;
    return info;
}

void Shower::setWaterTemperature(uint8_t temperature)
{
    waterTemperature.temperature = temperature;
    waterTemperature.time = timeutil::formatted();
    waterTemperature.user = whoNow;
    Serial.printf("[Shower %u] T=%u user=%s at %s\n",
                  id, temperature, whoNow.c_str(), waterTemperature.time.c_str());
}

String Shower::getWaterTemperature() const     { return waterTemperature.getInfo(); }
void   Shower::setWhoNow(const String &id_)    { whoNow = id_; }
String Shower::getWhoNow() const               { return whoNow; }
void   Shower::clearWhoNow()                   { whoNow = ""; }
bool   Shower::isBusyNow() const               { return isBusy; }
