
// Example sketch which shows how to display some patterns
// on a 64x32 LED matrix
//

#include <ESP32-HUB75-MatrixPanel-I2S-DMA.h>
#include "pool-screen.h"

void setup() {
  InitParams params;
  params.Pins = nullptr;
  poolScreenInit(&params);

  poolScreenClear();
  poolScreenLog("Find wifi..\n");
  delay(1500);
  poolScreenLog("Found wifi\n");
  delay(1500);
  poolScreenLog("Find sensors..\n");
  delay(1500);
  poolScreenLog("Found sensors\n");
  delay(1500);
}

double pool = 38.5;
double deltaT = 0.0;
double boiler = 30.5;
double heatExchangerIn = 60.3;
double heatExchangerOut = 55.8;

class MockUpdater
{
  unsigned long m_interval;
  unsigned long m_nextUpdate = 0;
  double m_updateMin;
  double m_updateMax;

public:
  MockUpdater(unsigned long interval, double updateMin, double updateMax)
    : m_interval(interval)
    , m_updateMin(updateMin)
    , m_updateMax(updateMax)
  {
  }

  double update(unsigned long curMillis, double old)
  {
    if (curMillis < m_nextUpdate)
    {
      return old;
    }

    m_nextUpdate = curMillis + m_interval;
    double update = m_updateMin + (m_updateMax - m_updateMin) * (random(10000) / 10000.0);
    return old + update;
  }
};

MockUpdater poolUpdater(5000, -0.5, 0.5);
MockUpdater boilerUpdater(5000, -0.5, 0.5);
MockUpdater heatExchangerInUpdater(5000, -0.5, 0.5);
MockUpdater heatExchangerOutUpdater(5000, -0.5, 0.5);

double lastPoolTemp = pool;
unsigned long lastDeltaTComputation = 0;
unsigned long deltaTComputationInterval = 1000 * 25;

void loop() {

    unsigned long curMillis = millis();

    pool = poolUpdater.update(curMillis, pool);
    boiler = boilerUpdater.update(curMillis, boiler);
    heatExchangerIn = heatExchangerInUpdater.update(curMillis, heatExchangerIn);
    heatExchangerOut = heatExchangerOutUpdater.update(curMillis, heatExchangerOut);

    if (curMillis >= lastDeltaTComputation + deltaTComputationInterval)
    {
      double elapsedHours = (curMillis - lastDeltaTComputation) * (1.0 / (1000.0 * 60.0 * 60.0));
      deltaT = (pool - lastPoolTemp) / elapsedHours;
      lastDeltaTComputation = curMillis;
      lastPoolTemp = pool;
    }

    DrawParams params;
    params.PoolIn = pool;
    params.PoolInDeltaT = deltaT;
    params.Boiler = boiler;
    params.HeatExchangerIn = heatExchangerIn;
    params.HeatExchangerOut = heatExchangerOut;
    poolScreenDraw(&params);

    delay(500); 
}