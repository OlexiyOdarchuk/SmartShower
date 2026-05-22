#pragma once
#include <Arduino.h>
#include <time.h>

// Європа/Київ з автоматичним DST:
// EET = UTC+2 (зима), EEST = UTC+3 (літо).
// DST вмикається в останню неділю березня о 03:00, вимикається — в останню неділю жовтня о 04:00.
namespace timeutil
{
inline void setup()
{
    configTzTime("EET-2EEST,M3.5.0/3,M10.5.0/4",
                 "0.ua.pool.ntp.org", "1.ua.pool.ntp.org", "pool.ntp.org");
}

inline bool isSet()
{
    time_t now = time(nullptr);
    return now > 1577836800; // > 2020-01-01 — значить реальний час встановлено
}

inline uint8_t hour()
{
    struct tm t;
    if (!getLocalTime(&t, 0)) return 0;
    return t.tm_hour;
}

inline String formatted()
{
    struct tm t;
    if (!getLocalTime(&t, 0)) return String("--:--:--");
    char buf[16];
    strftime(buf, sizeof(buf), "%H:%M:%S", &t);
    return String(buf);
}
} // namespace timeutil
