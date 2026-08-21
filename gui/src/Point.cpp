// NebulaOS GUI - Point Implementation
// ====================================
//
// Implementation of 2D Point class

#include "../include/Point.h"
#include "../include/GuiTypes.h"
#include "../../lib/include/string.h"

namespace NebulaOS {
namespace GUI {

// -----------------------------------------------------------------------------
// ToString - Convert point to string representation
// -----------------------------------------------------------------------------
void Point::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    snprintf(buffer, size, "Point(%d, %d)", x, y);
}

} // namespace GUI
} // namespace NebulaOS
