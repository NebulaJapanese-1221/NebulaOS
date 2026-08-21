// NebulaOS GUI - Color Implementation
// ====================================
//
// Implementation of Color class

#include "../include/Color.h"
#include "../include/GuiTypes.h"
#include "../../lib/include/string.h"
#include "../../kernel/common/include/nebula.h"

namespace NebulaOS {
namespace GUI {

// -----------------------------------------------------------------------------
// ToString - Convert color to string representation
// -----------------------------------------------------------------------------
void Color::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    snprintf(buffer, size, "Color(%d, %d, %d, %d)", 
             components.r, components.g, components.b, components.a);
}

// -----------------------------------------------------------------------------
// Helper functions for internal use
// -----------------------------------------------------------------------------

static inline float min3(float a, float b, float c) {
    float result = a;
    if (b < result) result = b;
    if (c < result) result = c;
    return result;
}

static inline float max3(float a, float b, float c) {
    float result = a;
    if (b > result) result = b;
    if (c > result) result = c;
    return result;
}

static inline uint8_t clamp8(int32_t value) {
    return (uint8_t)(value < 0 ? 0 : (value > 255 ? 255 : value));
}

} // namespace GUI
} // namespace NebulaOS
