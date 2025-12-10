#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <GyverOLED.h>
#include <WiFi.h>
#define NOTE_B0 31
#define NOTE_C1 33
#define NOTE_CS1 35
#define NOTE_D1 37
#define NOTE_DS1 39
#define NOTE_E1 41
#define NOTE_F1 44
#define NOTE_FS1 46
#define NOTE_G1 49
#define NOTE_GS1 52
#define NOTE_A1 55
#define NOTE_AS1 58
#define NOTE_B1 62
#define NOTE_C2 65
#define NOTE_CS2 69
#define NOTE_D2 73
#define NOTE_DS2 78
#define NOTE_E2 82
#define NOTE_F2 87
#define NOTE_FS2 93
#define NOTE_G2 98
#define NOTE_GS2 104
#define NOTE_A2 110
#define NOTE_AS2 117
#define NOTE_B2 123
#define NOTE_C3 131
#define NOTE_CS3 139
#define NOTE_D3 147
#define NOTE_DS3 156
#define NOTE_E3 165
#define NOTE_F3 175
#define NOTE_FS3 185
#define NOTE_G3 196
#define NOTE_GS3 208
#define NOTE_A3 220
#define NOTE_AS3 233
#define NOTE_B3 247
#define NOTE_C4 262
#define NOTE_CS4 277
#define NOTE_D4 294
#define NOTE_DS4 311
#define NOTE_E4 330
#define NOTE_F4 349
#define NOTE_FS4 370
#define NOTE_G4 392
#define NOTE_GS4 415
#define NOTE_A4 440
#define NOTE_AS4 466
#define NOTE_B4 494
#define NOTE_C5 523
#define NOTE_CS5 554
#define NOTE_D5 587
#define NOTE_DS5 622
#define NOTE_E5 659
#define NOTE_F5 698
#define NOTE_FS5 740
#define NOTE_G5 784
#define NOTE_GS5 831
#define NOTE_A5 880
#define NOTE_AS5 932
#define NOTE_B5 988
#define NOTE_C6 1047
#define NOTE_CS6 1109
#define NOTE_D6 1175
#define NOTE_DS6 1245
#define NOTE_E6 1319
#define NOTE_F6 1397
#define NOTE_FS6 1480
#define NOTE_G6 1568
#define NOTE_GS6 1661
#define NOTE_A6 1760
#define NOTE_AS6 1865
#define NOTE_B6 1976
#define NOTE_C7 2093
#define NOTE_CS7 2217
#define NOTE_D7 2349
#define NOTE_DS7 2489
#define NOTE_E7 2637
#define NOTE_F7 2794
#define NOTE_FS7 2960
#define NOTE_G7 3136
#define NOTE_GS7 3322
#define NOTE_A7 3520
#define NOTE_AS7 3729
#define NOTE_B7 3951
#define NOTE_C8 4186
#define NOTE_CS8 4435
#define NOTE_D8 4699
#define NOTE_DS8 4978
#define REST 0

int tempo = 144;

int buzzer = BUZZER;

int melody[] = {

    REST,
    2,
    NOTE_D4,
    4,
    NOTE_G4,
    -4,
    NOTE_AS4,
    8,
    NOTE_A4,
    4,
    NOTE_G4,
    2,
    NOTE_D5,
    4,
    NOTE_C5,
    -2,
    NOTE_A4,
    -2,
    NOTE_G4,
    -4,
    NOTE_AS4,
    8,
    NOTE_A4,
    4,
    NOTE_F4,
    2,
    NOTE_GS4,
    4,
    NOTE_D4,
    -1,
    NOTE_D4,
    4,

    NOTE_G4,
    -4,
    NOTE_AS4,
    8,
    NOTE_A4,
    4, // 10
    NOTE_G4,
    2,
    NOTE_D5,
    4,
    NOTE_F5,
    2,
    NOTE_E5,
    4,
    NOTE_DS5,
    2,
    NOTE_B4,
    4,
    NOTE_DS5,
    -4,
    NOTE_D5,
    8,
    NOTE_CS5,
    4,
    NOTE_CS4,
    2,
    NOTE_B4,
    4,
    NOTE_G4,
    -1,
    NOTE_AS4,
    4,

    NOTE_D5,
    2,
    NOTE_AS4,
    4, // 18
    NOTE_D5,
    2,
    NOTE_AS4,
    4,
    NOTE_DS5,
    2,
    NOTE_D5,
    4,
    NOTE_CS5,
    2,
    NOTE_A4,
    4,
    NOTE_AS4,
    -4,
    NOTE_D5,
    8,
    NOTE_CS5,
    4,
    NOTE_CS4,
    2,
    NOTE_D4,
    4,
    NOTE_D5,
    -1,
    REST,
    4,
    NOTE_AS4,
    4,

    NOTE_D5,
    2,
    NOTE_AS4,
    4, // 26
    NOTE_D5,
    2,
    NOTE_AS4,
    4,
    NOTE_F5,
    2,
    NOTE_E5,
    4,
    NOTE_DS5,
    2,
    NOTE_B4,
    4,
    NOTE_DS5,
    -4,
    NOTE_D5,
    8,
    NOTE_CS5,
    4,
    NOTE_CS4,
    2,
    NOTE_AS4,
    4,
    NOTE_G4,
    -1,

};
int notes = sizeof(melody) / sizeof(melody[0]) / 2;

int wholenote = (60000 * 4) / tempo;

int divider = 0, noteDuration = 0;

GyverOLED<SSD1306_128x32, OLED_BUFFER> oled;

Shower shower1(SHOWER_1_TIMER_DIO, SHOWER_1_TIMER_CLK, SHOWER_1_BUTTON, SHOWER_1_GROUND);
Shower shower2(SHOWER_2_TIMER_DIO, SHOWER_2_TIMER_CLK, SHOWER_2_BUTTON, SHOWER_2_GROUND);

int rows[2] = {SHOWER_1_GROUND, SHOWER_2_GROUND};
int cols[4] = {SHOWER_TEMPERATURE_BUTTON_1, SHOWER_TEMPERATURE_BUTTON_2, SHOWER_TEMPERATURE_BUTTON_3, SHOWER_TEMPERATURE_BUTTON_4};

void setup()
{
    Serial.begin(115200);
    WiFi.begin(WIFI_SSID, WIFI_PASS);
    while (WiFi.status() != WL_CONNECTED)
    {
        delay(500);
        Serial.print(".");
    }
    Serial.println("Connected");
    Serial.println(WiFi.localIP());
    pinMode(SHOWER_1_TIMER_DIO, OUTPUT);
    pinMode(SHOWER_1_TIMER_CLK, OUTPUT);
    pinMode(SHOWER_1_BUTTON, INPUT_PULLUP);
    oled.init(OLED_SDA, OLED_SCK);
    Wire.setClock(800000L); // макс. 800'000
    for (int r = 0; r < 2; r++)
    {
        pinMode(shower1, OUTPUT);
        digitalWrite(rows[r], HIGH);
    }

    for (int c = 0; c < 4; c++)
    {
        pinMode(cols[c], INPUT_PULLUP);
    }
    pinMode(SHOWER_1_RED_LED, OUTPUT);
    pinMode(SHOWER_2_RED_LED, OUTPUT);
    pinMode(SHOWER_1_GREEN_LED, OUTPUT);
    pinMode(SHOWER_2_GREEN_LED, OUTPUT);

    pinMode(BLACK_BUTTON, INPUT_PULLUP);
    pinMode(QUEUE_BUTTON, INPUT_PULLUP);

    digitalWrite(SHOWER_1_RED_LED, LOW);
    digitalWrite(SHOWER_2_RED_LED, LOW);
    digitalWrite(SHOWER_1_GREEN_LED, LOW);
    digitalWrite(SHOWER_2_GREEN_LED, LOW);

    for (int thisNote = 0; thisNote < notes * 2; thisNote = thisNote + 2)
    {

        divider = melody[thisNote + 1];
        if (divider > 0)
        {
            noteDuration = (wholenote) / divider;
        }
        else if (divider < 0)
        {
            noteDuration = (wholenote) / abs(divider);
            noteDuration *= 1.5;
        }

        tone(buzzer, melody[thisNote], noteDuration * 0.9);

        delay(noteDuration);

        noTone(buzzer);
    }
}

void loop()
{
    uint32_t tmr;

    tmr = millis();
    while (millis() - tmr < 5000)
        net();

    tmr = millis();
    while (millis() - tmr < 5000)
        bigBall();

    tmr = millis();
    while (millis() - tmr < 5000)
        lines();

    tmr = millis();
    while (millis() - tmr < 5000)
        circleModes();

    tmr = millis();
    while (millis() - tmr < 5000)
        ball();

    tmr = millis();
    while (millis() - tmr < 5000)
        bezier();

    tmr = millis();
    while (millis() - tmr < 5000)
        bezier2();
    shower1.updateDisplay();
    shower2.updateDisplay();
    temperatureButton();
}
void chekButtons()
{
    if (digitalRead(QUEUE_BUTTON) == LOW)
    {
        Serial.println("QUEUE_BUTTON");
    }
    if (digitalRead(BLACK_BUTTON) == LOW)
    {
        Serial.println("BLACK_BUTTON");
    }
}
void bezier2()
{
    const byte amount = 3;
    static bool start = false;
    static int x[amount], y[amount];
    static int velX[amount], velY[amount];
    if (!start)
    {
        start = 1;
        for (byte i = 0; i < amount; i++)
        {
            x[i] = random(10, (128 - 1) * 10);
            y[i] = random(10, (64 - 1) * 10);
            velX[i] = random(5, 20);
            velY[i] = random(5, 20);
        }
    }
    oled.clear();
    int bez[(amount + 1) * 2];
    for (byte i = 0; i < amount; i++)
    {
        x[i] += velX[i];
        y[i] += velY[i];
        if (x[i] >= (128 - 1) * 10 || x[i] < 0)
            velX[i] = -velX[i];
        if (y[i] >= (64 - 1) * 10 || y[i] < 0)
            velY[i] = -velY[i];
        oled.dot(x[i] / 10, y[i] / 10, 1);
        bez[i * 2] = x[i] / 10;
        bez[i * 2 + 1] = y[i] / 10;
    }
    bez[amount * 2] = bez[0];
    bez[amount * 2 + 1] = bez[1];

    oled.bezier(bez, amount + 1, 8);
    oled.update();
}

void bezier()
{
    int data[] = {0, 0, 128 / 2, 64 / 2, 0, 64 - 1};
    for (int i = 0; i < 128; i++)
    {
        oled.clear();
        data[0] = data[4] = 128 - i;
        data[2] = i;
        oled.bezier(data, 3, 7);
        oled.update();
    }
    for (int i = 128; i > 0; i--)
    {
        oled.clear();
        data[0] = data[4] = 128 - i;
        data[2] = i;
        oled.bezier(data, 3, 7);
        oled.update();
    }
}

void ball()
{
    oled.clear();
    static int x, y;
    static int velX = 5, velY = 8;
    x += velX;
    y += velY;
    if (x >= (128 - 1) * 10 || x < 0)
        velX = -velX;
    if (y >= (64 - 1) * 10 || y < 0)
        velY = -velY;

    oled.dot(x / 10, y / 10, 1);
    oled.dot(x / 10 + 1, y / 10 + 1, 1);
    oled.dot(x / 10 + 1, y / 10, 1);
    oled.dot(x / 10, y / 10 + 1, 1);
    oled.update();
    delay(10);
}

void net()
{
    const byte radius = 3;
    const byte amount = 6;
    static bool start = false;
    static int x[amount], y[amount];
    static int velX[amount], velY[amount];
    if (!start)
    {
        start = 1;
        for (byte i = 0; i < amount; i++)
        {
            x[i] = random(10, (128 - 1) * 10);
            y[i] = random(10, (64 - 1) * 10);
            velX[i] = random(2, 9);
            velY[i] = random(2, 9);
        }
    }
    oled.clear();
    for (byte i = 0; i < amount; i++)
    {
        x[i] += velX[i];
        y[i] += velY[i];
        if (x[i] >= (128 - 1 - radius) * 10 || x[i] < radius * 10)
            velX[i] = -velX[i];
        if (y[i] >= (64 - 1 - radius) * 10 || y[i] < radius * 10)
            velY[i] = -velY[i];
        oled.circle(x[i] / 10, y[i] / 10, radius);
    }

    for (int i = 0; i < amount; i++)
    {
        for (int j = 0; j < amount; j++)
        {
            if (i != j && dist(x[i] / 10, y[i] / 10, x[j] / 10, y[j] / 10) < 35)
                oled.line(x[i] / 10, y[i] / 10, x[j] / 10, y[j] / 10);
        }
    }
    oled.update();
}

int dist(int x1, int y1, int x2, int y2)
{
    int lx = (x2 - x1);
    int ly = (y2 - y1);
    return (sqrt(lx * lx + ly * ly));
}

void bigBall()
{
    oled.clear();
    byte radius = 10;
    static int x = (128 / 2) * 10, y = (64 / 2) * 10;
    static int velX = 17, velY = 9;
    static bool fillFlag = 0;
    x += velX;
    y += velY;
    if (x >= (128 - radius) * 10 || x < radius * 10)
    {
        velX = -velX;
        fillFlag = !fillFlag;
    }
    if (y >= (64 - radius) * 10 || y < radius * 10)
    {
        velY = -velY;
        fillFlag = !fillFlag;
    }

    oled.circle(x / 10, y / 10, radius, fillFlag ? OLED_STROKE : OLED_FILL);
    oled.update();
}

void lines()
{
    oled.clear();
    for (byte i = 0; i < 128 - 1; i += 3)
    {
        oled.line(0, 0, i, 64);
        oled.update();
    }
    for (int i = 64 - 1; i >= 0; i -= 3)
    {
        oled.line(0, 0, 128, i);
        oled.update();
    }
    delay(100);

    oled.clear();
    for (int i = 128 - 1; i > 0; i -= 3)
    {
        oled.line(128 - 1, 0, i, 64);
        oled.update();
    }
    for (int i = 64 - 1; i > 0; i -= 3)
    {
        oled.line(128 - 1, 0, 0, i);
        oled.update();
    }
    delay(100);
}

void circleModes()
{
    oled.clear();
    oled.fill(255);
    oled.createBuffer(64 - 20, 32 - 20, 64 + 20, 32 + 20, 255);
    oled.circle(64, 32, 20, OLED_CLEAR);
    oled.sendBuffer();
    oled.update();
    delay(800);

    oled.clear();
    oled.createBuffer(64 - 20, 32 - 20, 64 + 20, 32 + 20);
    oled.circle(64, 32, 20, OLED_FILL);
    oled.sendBuffer();
    oled.update();
    delay(800);

    oled.clear();
    oled.createBuffer(64 - 20, 32 - 20, 64 + 20, 32 + 20);
    oled.circle(64, 32, 20, OLED_STROKE);
    oled.sendBuffer();
    oled.update();
    delay(800);
}

class Shower
{
private:
    u8_t button;
    ulong start = 0;
    bool isRunning = false;
    Disp1637Colon showerTimer;

public:
    Shower::Shower(u8_t displayDIO, u8_t displayCLK, u8_t button, u8_t temperatureGround)
        : showerTimer(displayDIO, displayCLK), button(button) {};

    void updateDisplay()
    {
        showerTimer.setCursor(0);
        if (!digitalRead(button) == false)
        {
            isRunning = false;
            showerTimer.print("FREE");
            showerTimer.update();
            return;
        }
        if (!digitalRead(button) != isRunning)
        {
            isRunning = true;
            start = millis();
        }
        unsigned long totalMs = millis() - start;

        u8_t minutes = (totalMs / (1000 * 60)) % 60;
        u8_t seconds = (totalMs / 1000) % 60;

        char timeBuffer[6];

        sprintf(timeBuffer, "%02d%02d", minutes, seconds);

        showerTimer.print(timeBuffer);
        showerTimer.colon(true);
        showerTimer.update();
    };
};

void temperatureButton()
{
    for (int r = 0; r < 2; r++)
    {

        digitalWrite(rows[r], LOW);

        for (int c = 0; c < 4; c++)
        {
            if (digitalRead(cols[c]) == LOW)
            {
                Serial.print("Pressed: R");
                Serial.print(r);
                Serial.print(" C");
                Serial.println(c);
                delay(200);
            }
        }

        digitalWrite(rows[r], HIGH);
    }
}