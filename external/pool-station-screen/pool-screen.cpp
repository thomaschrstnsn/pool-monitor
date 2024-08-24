#include <ESP32-HUB75-MatrixPanel-I2S-DMA.h>
#include "pool-screen.h"

static MatrixPanel_I2S_DMA *display;

#define PANEL_RES_X 64      // Number of pixels wide of each INDIVIDUAL panel module. 
#define PANEL_RES_Y 64     // Number of pixels tall of each INDIVIDUAL panel module.
#define PANEL_CHAIN 2      // Total number of panels chained one to another

static void printTemp(double temp, int16_t size, uint16_t color)
{
  char text[32];
  snprintf(text, sizeof(text), "%.1f", temp);

  display->setTextSize(size);
  display->setTextColor(color);
  for (char c : text)
  {
    if (c == '\0')
    {
      break;
    }

    if (c == '.')
    {
      int pad = size == 1 ? 0 : 1;
      display->fillRect(display->getCursorX() - pad, display->getCursorY() + static_cast<int16_t>(size * 7) - size, size, size, color);
      display->setCursor(display->getCursorX() + 2 * size - 2 * pad, display->getCursorY());
    }
    else
    {
      display->print(c);
    }
  }

  int degreeVerticalOffset = size == 1 ? 0 : 1;
  display->drawRoundRect(display->getCursorX() - 1, display->getCursorY() + degreeVerticalOffset, size + 2, size + 2, 1, color);
}

static void drawTemps(double poolIn, double deltaT, double boiler, double heatChangerIn, double heatChangerOut)
{
  display->setTextWrap(false);

  int16_t leftCol = 1;
  display->setCursor(leftCol, 1);

  printTemp(poolIn, 6, display->color444(15, 4, 0));

  int16_t afterPoolTemp = 1 + (7 * 6) + 2;
  display->drawLine(0, afterPoolTemp, PANEL_RES_X * PANEL_CHAIN, afterPoolTemp, display->color444(15, 15, 15));
  display->setCursor(leftCol, afterPoolTemp + 2);

  display->setTextSize(1);

  display->setTextColor(display->color444(4, 15, 15));
  display->print("Kedel:");
  printTemp(boiler, 1, display->color444(0, 15, 0));
  display->setCursor(display->getCursorX() + 7, display->getCursorY());

  display->setTextColor(display->color444(15, 4, 4));
  display->drawLine(display->getCursorX(), display->getCursorY() + 6, display->getCursorX() + 4, display->getCursorY(), display->color444(15, 4, 4));
  display->drawLine(display->getCursorX() + 4, display->getCursorY(), display->getCursorX() + 8, display->getCursorY() + 6, display->color444(15, 4, 4));
  display->drawLine(display->getCursorX(), display->getCursorY() + 6, display->getCursorX() + 8, display->getCursorY() + 6, display->color444(15, 4, 4));

  display->setCursor(display->getCursorX() + 9, display->getCursorY());
  display->print("T:");
  printTemp(deltaT, 1, display->color444(15, 4, 0));
  display->setCursor(display->getCursorX() + 2, display->getCursorY());
  display->print("/h");

  display->setCursor(leftCol, afterPoolTemp + 2 + 7 + 2);
  display->setTextColor(display->color444(4, 15, 15));
  display->print("Veksler");
  display->setCursor(display->getCursorX() + 3, display->getCursorY());
  display->print("I/O:");
  printTemp(heatChangerIn, 1, display->color444(0, 15, 0));
  display->setTextColor(display->color444(4, 15, 15));
  display->setCursor(display->getCursorX() + 2, display->getCursorY());
  display->print("/");
  printTemp(heatChangerOut, 1, display->color444(0, 15, 0));
}

static int8_t BreakoutTestBoardPins[] =
{
    25, // R1_PIN
    26, // G1_PIN
    27, // B1_PIN
    14, // R2_PIN
    12, // G2_PIN
    13, // B2_PIN
    23, // A_PIN
    22, // B_PIN
    5,  // C_PIN
    34, // 17 // D_PIN
    32, // E_PIN
    4, // LAT_PIN
    15, // OE_PIN
    2, // 16 // CLK_PIN
};

static int8_t TestBoardPins[] =
{
    25, // R1_PIN
    26, // G1_PIN
    27, // B1_PIN
    14, // R2_PIN
    12, // G2_PIN
    13, // B2_PIN
    23, // A_PIN
    22, // B_PIN
    5,  // C_PIN
    17, // D_PIN
    32, // E_PIN
    4,  // LAT_PIN
    15, // OE_PIN
    16, // CLK_PIN
};

void poolScreenInit(const InitParams* params)
{
  int8_t* pins = params->Pins == nullptr ? TestBoardPins : params->Pins;
  HUB75_I2S_CFG::i2s_pins pinsStruct = {
    pins[0], pins[1], pins[2], pins[3],
    pins[4], pins[5], pins[6], pins[7],
    pins[8], pins[9], pins[10], pins[11],
    pins[12], pins[13]};

  // Module configuration
  HUB75_I2S_CFG mxconfig(
    PANEL_RES_X,   // module width
    PANEL_RES_Y,   // module height
    PANEL_CHAIN,   // Chain length
    pinsStruct
  );

  // Display Setup
  display = new MatrixPanel_I2S_DMA(mxconfig);
  display->begin();
  display->setBrightness8(128); //0-255
  display->clearScreen();
  display->fillScreen(display->color565(255, 255, 255));
  
  // fix the screen with green
  display->fillRect(0, 0, display->width(), display->height(), display->color444(0, 15, 0));
  delay(500);

  // draw a box in yellow
  display->drawRect(0, 0, display->width(), display->height(), display->color444(15, 15, 0));
  delay(500);

  // draw an 'X' in red
  display->drawLine(0, 0, display->width()-1, display->height()-1, display->color444(15, 0, 0));
  display->drawLine(display->width()-1, 0, 0, display->height()-1, display->color444(15, 0, 0));
  delay(500);

  // draw a blue circle
  display->drawCircle(10, 10, 10, display->color444(0, 0, 15));
  delay(500);

  // fill a violet circle
  display->fillCircle(40, 21, 10, display->color444(15, 0, 15));
  delay(500);

  // fill the screen with 'black'
  display->fillScreen(display->color444(0, 0, 0));
}

void poolScreenDraw(const DrawParams* params)
{
  display->clearScreen();
  drawTemps(params->PoolIn, params->PoolInDeltaT, params->Boiler, params->HeatExchangerIn, params->HeatExchangerOut);
}

void poolScreenClear()
{
  display->clearScreen();
  display->setCursor(0, 0);
}

void poolScreenLog(const char* text)
{
  display->setTextWrap(true);
  display->setTextSize(1);

  unsigned long curMillis = millis();
  char prefix[32];
  snprintf(prefix, sizeof(prefix), "[%.1f] ", (double)curMillis / 1000);
  display->print(prefix);
  display->print(text);
}