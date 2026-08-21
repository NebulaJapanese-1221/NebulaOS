// NebulaOS GUI - Rectangle Class
// ================================
//
// Rectangle class for GUI regions

#ifndef NEBULAOS_GUI_RECTANGLE_H
#define NEBULAOS_GUI_RECTANGLE_H

#include "../include/Point.h"
#include "../include/Size.h"
#include "../../kernel/common/include/nebula.h"

namespace NebulaOS {
namespace GUI {

class Rectangle {
public:
    int32_t x;
    int32_t y;
    int32_t width;
    int32_t height;

    // Constructors
    Rectangle() : x(0), y(0), width(0), height(0) {}
    Rectangle(int32_t x, int32_t y, int32_t width, int32_t height) 
        : x(x), y(y), width(width), height(height) {}
    Rectangle(const Point& location, const Size& size) 
        : x(location.x), y(location.y), width(size.width), height(size.height) {}
    Rectangle(const Rectangle& other) 
        : x(other.x), y(other.y), width(other.width), height(other.height) {}

    // Operator overloads
    Rectangle& operator=(const Rectangle& other) {
        x = other.x;
        y = other.y;
        width = other.width;
        height = other.height;
        return *this;
    }

    bool operator==(const Rectangle& other) const {
        return x == other.x && y == other.y && 
               width == other.width && height == other.height;
    }

    bool operator!=(const Rectangle& other) const {
        return !(*this == other);
    }

    // Utility methods
    void Set(int32_t newX, int32_t newY, int32_t newWidth, int32_t newHeight) {
        x = newX;
        y = newY;
        width = newWidth;
        height = newHeight;
    }

    void SetLocation(int32_t newX, int32_t newY) {
        x = newX;
        y = newY;
    }

    void SetSize(int32_t newWidth, int32_t newHeight) {
        width = newWidth;
        height = newHeight;
    }

    Point GetLocation() const { return Point(x, y); }
    Size GetSize() const { return Size(width, height); }

    int32_t GetLeft() const { return x; }
    int32_t GetTop() const { return y; }
    int32_t GetRight() const { return x + width; }
    int32_t GetBottom() const { return y + height; }

    int32_t GetCenterX() const { return x + width / 2; }
    int32_t GetCenterY() const { return y + height / 2; }
    Point GetCenter() const { return Point(GetCenterX(), GetCenterY()); }

    bool Contains(int32_t px, int32_t py) const {
        return px >= x && px < x + width && py >= y && py < y + height;
    }

    bool Contains(const Point& point) const {
        return Contains(point.x, point.y);
    }

    bool Intersects(const Rectangle& other) const {
        return !(other.x >= x + width ||
                 other.x + other.width <= x ||
                 other.y >= y + height ||
                 other.y + other.height <= y);
    }

    Rectangle Intersection(const Rectangle& other) const {
        int32_t left = MAX(x, other.x);
        int32_t top = MAX(y, other.y);
        int32_t right = MIN(x + width, other.x + other.width);
        int32_t bottom = MIN(y + height, other.y + other.height);
        
        if (right < left || bottom < top) {
            return Rectangle();
        }
        return Rectangle(left, top, right - left, bottom - top);
    }

    Rectangle Union(const Rectangle& other) const {
        int32_t left = MIN(x, other.x);
        int32_t top = MIN(y, other.y);
        int32_t right = MAX(x + width, other.x + other.width);
        int32_t bottom = MAX(y + height, other.y + other.height);
        return Rectangle(left, top, right - left, bottom - top);
    }

    void Inflate(int32_t dx, int32_t dy) {
        x -= dx;
        y -= dy;
        width += 2 * dx;
        height += 2 * dy;
    }

    void Deflate(int32_t dx, int32_t dy) {
        x += dx;
        y += dy;
        width -= 2 * dx;
        height -= 2 * dy;
        if (width < 0) width = 0;
        if (height < 0) height = 0;
    }

    void Offset(int32_t dx, int32_t dy) {
        x += dx;
        y += dy;
    }

    bool IsEmpty() const {
        return width <= 0 || height <= 0;
    }

    // Static methods
    static Rectangle Empty() { return Rectangle(0, 0, 0, 0); }

    // Serialization
    void ToString(char* buffer, size_t size) const;
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_RECTANGLE_H
