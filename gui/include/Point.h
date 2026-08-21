// NebulaOS GUI - Point Class
// ===========================
//
// 2D point class for GUI coordinate system

#ifndef NEBULAOS_GUI_POINT_H
#define NEBULAOS_GUI_POINT_H

#include "../include/GuiTypes.h"

namespace NebulaOS {
namespace GUI {

class Point {
public:
    int32_t x;
    int32_t y;

    // Constructors
    Point() : x(0), y(0) {}
    Point(int32_t x, int32_t y) : x(x), y(y) {}
    Point(const Point& other) : x(other.x), y(other.y) {}

    // Operator overloads
    Point& operator=(const Point& other) {
        x = other.x;
        y = other.y;
        return *this;
    }

    bool operator==(const Point& other) const {
        return x == other.x && y == other.y;
    }

    bool operator!=(const Point& other) const {
        return !(*this == other);
    }

    Point operator+(const Point& other) const {
        return Point(x + other.x, y + other.y);
    }

    Point operator-(const Point& other) const {
        return Point(x - other.x, y - other.y);
    }

    Point& operator+=(const Point& other) {
        x += other.x;
        y += other.y;
        return *this;
    }

    Point& operator-=(const Point& other) {
        x -= other.x;
        y -= other.y;
        return *this;
    }

    Point operator*(int32_t scalar) const {
        return Point(x * scalar, y * scalar);
    }

    Point operator/(int32_t scalar) const {
        return Point(x / scalar, y / scalar);
    }

    // Utility methods
    void Set(int32_t newX, int32_t newY) {
        x = newX;
        y = newY;
    }

    void Translate(int32_t dx, int32_t dy) {
        x += dx;
        y += dy;
    }

    int64_t DistanceTo(const Point& other) const {
        int64_t dx = x - other.x;
        int64_t dy = y - other.y;
        return dx * dx + dy * dy;  // Squared distance (avoid sqrt)
    }

    int32_t ManhattanDistanceTo(const Point& other) const {
        int32_t dx = x - other.x;
        int32_t dy = y - other.y;
        return (dx < 0 ? -dx : dx) + (dy < 0 ? -dy : dy);
    }

    // Static methods
    static Point Zero() { return Point(0, 0); }
    static Point One() { return Point(1, 1); }

    // Serialization
    void ToString(char* buffer, size_t size) const;
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_POINT_H
