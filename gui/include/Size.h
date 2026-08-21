// NebulaOS GUI - Size Class
// ==========================
//
// 2D size class for GUI dimensions

#ifndef NEBULAOS_GUI_SIZE_H
#define NEBULAOS_GUI_SIZE_H

#include "../include/GuiTypes.h"

namespace NebulaOS {
namespace GUI {

class Size {
public:
    int32_t width;
    int32_t height;

    // Constructors
    Size() : width(0), height(0) {}
    Size(int32_t width, int32_t height) : width(width), height(height) {}
    Size(const Size& other) : width(other.width), height(other.height) {}

    // Operator overloads
    Size& operator=(const Size& other) {
        width = other.width;
        height = other.height;
        return *this;
    }

    bool operator==(const Size& other) const {
        return width == other.width && height == other.height;
    }

    bool operator!=(const Size& other) const {
        return !(*this == other);
    }

    Size operator+(const Size& other) const {
        return Size(width + other.width, height + other.height);
    }

    Size operator-(const Size& other) const {
        return Size(width - other.width, height - other.height);
    }

    Size& operator+=(const Size& other) {
        width += other.width;
        height += other.height;
        return *this;
    }

    Size& operator-=(const Size& other) {
        width -= other.width;
        height -= other.height;
        return *this;
    }

    Size operator*(int32_t scalar) const {
        return Size(width * scalar, height * scalar);
    }

    Size operator/(int32_t scalar) const {
        return Size(width / scalar, height / scalar);
    }

    // Utility methods
    void Set(int32_t newWidth, int32_t newHeight) {
        width = newWidth;
        height = newHeight;
    }

    void Scale(int32_t factor) {
        width *= factor;
        height *= factor;
    }

    int64_t Area() const {
        return (int64_t)width * height;
    }

    bool IsEmpty() const {
        return width <= 0 || height <= 0;
    }

    // Static methods
    static Size Zero() { return Size(0, 0); }
    static Size One() { return Size(1, 1); }

    // Serialization
    void ToString(char* buffer, size_t size) const;
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_SIZE_H
