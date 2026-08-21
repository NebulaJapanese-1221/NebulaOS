// NebulaOS GUI - Color Class
// ==========================
//
// Color representation for GUI

#ifndef NEBULAOS_GUI_COLOR_H
#define NEBULAOS_GUI_COLOR_H

#include "../include/GuiTypes.h"
#include "../../kernel/common/include/nebula.h"

// For float min/max
static inline float fmin(float a, float b) { return a < b ? a : b; }
static inline float fmax(float a, float b) { return a > b ? a : b; }
static inline float fmin3(float a, float b, float c) { return fmin(fmin(a, b), c); }
static inline float fmax3(float a, float b, float c) { return fmax(fmax(a, b), c); }
static inline float fmod(float a, float b) { 
    int quotient = (int)(a / b);
    return a - quotient * b;
}
static inline float fabs(float a) { return a < 0 ? -a : a; }

namespace NebulaOS {
namespace GUI {

// Color in RGBA format (8 bits per channel)
class Color {
public:
    union {
        struct {
            uint8_t r;
            uint8_t g;
            uint8_t b;
            uint8_t a;
        } components;
        uint32_t value;
    };

    // Constructors
    Color() : value(0x00000000) {}
    Color(uint8_t r, uint8_t g, uint8_t b, uint8_t a = 255) {
        components.r = r;
        components.g = g;
        components.b = b;
        components.a = a;
    }
    Color(uint32_t rgba) : value(rgba) {}
    Color(const Color& other) : value(other.value) {}

    // Static color constants
    static Color Transparent() { return Color(0, 0, 0, 0); }
    static Color Black() { return Color(0, 0, 0); }
    static Color White() { return Color(255, 255, 255); }
    static Color Red() { return Color(255, 0, 0); }
    static Color Green() { return Color(0, 255, 0); }
    static Color Blue() { return Color(0, 0, 255); }
    static Color Yellow() { return Color(255, 255, 0); }
    static Color Cyan() { return Color(0, 255, 255); }
    static Color Magenta() { return Color(255, 0, 255); }
    static Color Gray() { return Color(128, 128, 128); }
    static Color LightGray() { return Color(192, 192, 192); }
    static Color DarkGray() { return Color(64, 64, 64); }
    static Color Orange() { return Color(255, 165, 0); }
    static Color Purple() { return Color(128, 0, 128); }
    static Color Brown() { return Color(165, 42, 42); }
    static Color Pink() { return Color(255, 192, 203); }

    // Operator overloads
    Color& operator=(const Color& other) {
        value = other.value;
        return *this;
    }

    bool operator==(const Color& other) const {
        return value == other.value;
    }

    bool operator!=(const Color& other) const {
        return value != other.value;
    }

    // Color operations
    Color operator+(const Color& other) const {
        return Color(
            fmin(components.r + other.components.r, 255),
            fmin(components.g + other.components.g, 255),
            fmin(components.b + other.components.b, 255),
            fmin(components.a + other.components.a, 255)
        );
    }

    Color operator-(const Color& other) const {
        return Color(
            fmax(components.r - other.components.r, 0),
            fmax(components.g - other.components.g, 0),
            fmax(components.b - other.components.b, 0),
            fmax(components.a - other.components.a, 0)
        );
    }

    Color operator*(float scalar) const {
        return Color(
            (uint8_t)fmin(components.r * scalar, 255.0f),
            (uint8_t)fmin(components.g * scalar, 255.0f),
            (uint8_t)fmin(components.b * scalar, 255.0f),
            (uint8_t)fmin(components.a * scalar, 255.0f)
        );
    }

    Color operator/(float scalar) const {
        return *this * (1.0f / scalar);
    }

    Color& operator+=(const Color& other) { *this = *this + other; return *this; }
    Color& operator-=(const Color& other) { *this = *this - other; return *this; }
    Color& operator*=(float scalar) { *this = *this * scalar; return *this; }
    Color& operator/=(float scalar) { *this = *this / scalar; return *this; }

    // Color manipulation
    uint8_t GetAlpha() const { return components.a; }
    Color WithAlpha(uint8_t alpha) const {
        Color result = *this;
        result.components.a = alpha;
        return result;
    }

    Color Lighten(float factor = 0.1f) const {
        return *this * (1.0f + factor);
    }

    Color Darken(float factor = 0.1f) const {
        return *this * (1.0f - factor);
    }

    Color Invert() const {
        return Color(255 - components.r, 255 - components.g, 255 - components.b, components.a);
    }

    // Convert to grayscale
    Color Grayscale() const {
        uint8_t gray = (uint8_t)((components.r * 0.299f) + (components.g * 0.587f) + (components.b * 0.114f));
        return Color(gray, gray, gray, components.a);
    }

    // Blend with another color
    Color Blend(const Color& other, float ratio = 0.5f) const {
        return Color(
            (uint8_t)(components.r * (1.0f - ratio) + other.components.r * ratio),
            (uint8_t)(components.g * (1.0f - ratio) + other.components.g * ratio),
            (uint8_t)(components.b * (1.0f - ratio) + other.components.b * ratio),
            (uint8_t)(components.a * (1.0f - ratio) + other.components.a * ratio)
        );
    }

    // Get brightness (0-1)
    float GetBrightness() const {
        return (components.r * 0.299f + components.g * 0.587f + components.b * 0.114f) / 255.0f;
    }

    // Get hue, saturation, value (HSV)
    void ToHSV(float& h, float& s, float& v) const {
        float r = components.r / 255.0f;
        float g = components.g / 255.0f;
        float b = components.b / 255.0f;

        float cmax = fmax3(r, g, b);
        float cmin = fmin3(r, g, b);
        float delta = cmax - cmin;

        // Value
        v = cmax;

        // Saturation
        if (cmax != 0) {
            s = delta / cmax;
        } else {
            s = 0;
        }

        // Hue
        if (delta == 0) {
            h = 0;
        } else if (cmax == r) {
            h = 60.0f * fmod((g - b) / delta, 6.0f);
        } else if (cmax == g) {
            h = 60.0f * (((b - r) / delta) + 2.0f);
        } else {
            h = 60.0f * (((r - g) / delta) + 4.0f);
        }

        if (h < 0) {
            h += 360.0f;
        }
    }

    // Create color from HSV
    static Color FromHSV(float h, float s, float v) {
        h = fmod(h, 360.0f);
        if (h < 0) h += 360.0f;

        float c = v * s;
        float x = c * (1.0f - fabs(fmod(h / 60.0f, 2.0f) - 1.0f));
        float m = v - c;

        float r, g, b;
        if (h < 60) {
            r = c; g = x; b = 0;
        } else if (h < 120) {
            r = x; g = c; b = 0;
        } else if (h < 180) {
            r = 0; g = c; b = x;
        } else if (h < 240) {
            r = 0; g = x; b = c;
        } else if (h < 300) {
            r = x; g = 0; b = c;
        } else {
            r = c; g = 0; b = x;
        }

        return Color(
            (uint8_t)((r + m) * 255.0f),
            (uint8_t)((g + m) * 255.0f),
            (uint8_t)((b + m) * 255.0f)
        );
    }

    // Convert to ARGB packed format (0xAARRGGBB)
    uint32_t ToARGB() const {
        return (components.a << 24) | (components.r << 16) | (components.g << 8) | components.b;
    }

    // Convert to ABGR packed format (0xAABBGGRR)
    uint32_t ToABGR() const {
        return (components.a << 24) | (components.b << 16) | (components.g << 8) | components.r;
    }

    // Convert to BGR format (0x00BBGGRR) - common for framebuffers
    uint32_t ToBGR() const {
        return (components.b << 16) | (components.g << 8) | components.r;
    }

    // Convert from ARGB packed format
    static Color FromARGB(uint32_t argb) {
        return Color(
            (uint8_t)((argb >> 16) & 0xFF),
            (uint8_t)((argb >> 8) & 0xFF),
            (uint8_t)(argb & 0xFF),
            (uint8_t)((argb >> 24) & 0xFF)
        );
    }

    // Convert from ABGR packed format
    static Color FromABGR(uint32_t abgr) {
        return Color(
            (uint8_t)(abgr & 0xFF),
            (uint8_t)((abgr >> 8) & 0xFF),
            (uint8_t)((abgr >> 16) & 0xFF),
            (uint8_t)((abgr >> 24) & 0xFF)
        );
    }

    // Convert from BGR format
    static Color FromBGR(uint32_t bgr) {
        return Color(
            (uint8_t)(bgr & 0xFF),
            (uint8_t)((bgr >> 8) & 0xFF),
            (uint8_t)((bgr >> 16) & 0xFF)
        );
    }

    // Serialization
    void ToString(char* buffer, size_t size) const;
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_COLOR_H
