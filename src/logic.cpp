#include <Arduino.h>
#include <secrets.h>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.h>
#include <logic.h>
#include <NTPClient.h>

WiFiUDP ntpUPD;
NTPClient timeClient(ntpUPD, "0.ua.pool.ntp.org", 2 * 3600, 60000);
Shower shower1(SHOWER_1_TIMER_DIO, SHOWER_1_TIMER_CLK, SHOWER_1_BUTTON, SHOWER_1_GROUND);
Shower shower2(SHOWER_2_TIMER_DIO, SHOWER_2_TIMER_CLK, SHOWER_2_BUTTON, SHOWER_2_GROUND);

// int rows[2] = {15, 4};
// int cols[4] = {16, 17, 5, 18};

// void setup()
// {
//     Serial.begin(115200);

//     for (int r = 0; r < 2; r++)
//     {
//         pinMode(rows[r], OUTPUT);
//         digitalWrite(rows[r], HIGH);
//     }

//     for (int c = 0; c < 4; c++)
//     {
//         pinMode(cols[c], INPUT_PULLUP);
//     }
// }

// void loop()
// {
//     for (int r = 0; r < 2; r++)
//     {

//         digitalWrite(rows[r], LOW);

//         for (int c = 0; c < 4; c++)
//         {
//             if (digitalRead(cols[c]) == LOW)
//             {
//                 Serial.print("Pressed: R");
//                 Serial.print(r);
//                 Serial.print(" C");
//                 Serial.println(c);
//                 delay(200);
//             }
//         }

//         digitalWrite(rows[r], HIGH);
//     }
// }