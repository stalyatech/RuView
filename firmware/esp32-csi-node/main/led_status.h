#ifndef LED_STATUS_H
#define LED_STATUS_H

typedef enum {
    LED_STATE_BOOT = 0,      /* slow blink: booting / connecting */
    LED_STATE_CONNECTED,     /* solid on: WiFi connected, IP acquired */
    LED_STATE_WIFI_FAIL,     /* fast blink: WiFi connect failed */
} led_state_t;

void led_status_init(void);
void led_status_set(led_state_t state);

#endif
