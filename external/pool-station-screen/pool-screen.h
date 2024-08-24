#pragma once

#include <cstdint>

extern "C" {
 
struct InitParams
{
    // nullptr to use default configuration of pins,
    // otherwise pointer to pin numbers for
    // R1_PIN
    // G1_PIN
    // B1_PIN
    // R2_PIN
    // G2_PIN
    // B2_PIN
    // A_PIN
    // B_PIN
    // C_PIN
    // D_PIN
    // E_PIN
    // LAT_PIN
    // OE_PIN
    // CLK_PIN

    int8_t* Pins;
};

struct DrawParams
{
    double PoolIn;
    double PoolInDeltaT;
    double Boiler;
    double HeatExchangerIn;
    double HeatExchangerOut;
};

void poolScreenInit(const InitParams* params);

void poolScreenDraw(const DrawParams* params);

void poolScreenClear();
void poolScreenLog(const char* text);

};