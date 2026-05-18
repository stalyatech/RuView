/* WS2812 RGB LED status driver for ESP32-S3 Zero (Waveshare).
 * Onboard NeoPixel-style RGB LED is on GPIO21. */

#include "led_status.h"
#include "esp_timer.h"
#include "led_strip.h"

#define LED_GPIO         21
#define BLINK_BOOT_US    500000   /* 1 Hz */
#define BLINK_FAIL_US    125000   /* 4 Hz */
#define BRIGHTNESS       16       /* dim — 6 % duty, plenty visible */

static led_state_t s_state = LED_STATE_BOOT;
static led_strip_handle_t s_strip;
static esp_timer_handle_t s_timer;
static uint8_t s_phase;

static void set_rgb(uint8_t r, uint8_t g, uint8_t b)
{
    if (s_strip == NULL) return;
    led_strip_set_pixel(s_strip, 0, r, g, b);
    led_strip_refresh(s_strip);
}

static void blink_cb(void *arg)
{
    (void)arg;
    switch (s_state) {
    case LED_STATE_BOOT:
        s_phase ^= 1;
        if (s_phase) set_rgb(0, 0, BRIGHTNESS); else set_rgb(0, 0, 0);
        esp_timer_start_once(s_timer, BLINK_BOOT_US);
        break;
    case LED_STATE_CONNECTED:
        set_rgb(0, BRIGHTNESS, 0);
        break;
    case LED_STATE_WIFI_FAIL:
        s_phase ^= 1;
        if (s_phase) set_rgb(BRIGHTNESS, 0, 0); else set_rgb(0, 0, 0);
        esp_timer_start_once(s_timer, BLINK_FAIL_US);
        break;
    }
}

void led_status_init(void)
{
    led_strip_config_t strip_cfg = {
        .strip_gpio_num = LED_GPIO,
        .max_leds = 1,
        .led_pixel_format = LED_PIXEL_FORMAT_GRB,
        .led_model = LED_MODEL_WS2812,
    };
    led_strip_rmt_config_t rmt_cfg = {
        .resolution_hz = 10 * 1000 * 1000,
    };
    if (led_strip_new_rmt_device(&strip_cfg, &rmt_cfg, &s_strip) != ESP_OK) {
        s_strip = NULL;
        return;
    }
    set_rgb(0, 0, 0);
    s_phase = 0;

    const esp_timer_create_args_t args = {
        .callback = blink_cb,
        .name = "led_blink",
    };
    esp_timer_create(&args, &s_timer);
    esp_timer_start_once(s_timer, BLINK_BOOT_US);
}

void led_status_set(led_state_t state)
{
    if (s_state == state) return;
    s_state = state;
    esp_timer_stop(s_timer);
    s_phase = 0;
    blink_cb(NULL);
}
