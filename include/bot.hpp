#pragma once
#include <Arduino.h>
#include <FastBot2.h>
#include <secrets.hpp>

#define FB_NO_FILE

extern FastBot2 bot;

void updateh(fb::Update &u);
void getInfoMessage(const int32_t messageID, const fb::ID chatID = GROUP_ID);
void addToQueueMessage(const String &id, const int32_t messageID, const fb::ID chatID = GROUP_ID);
void queueReductionMessage(const String &id, const int32_t messageID, const fb::ID chatID = GROUP_ID);
void queueReductionMessage(const fb::ID chatID = GROUP_ID);
fb::ReplyParam createReply(const int32_t messageID, const fb::ID chatID = GROUP_ID);