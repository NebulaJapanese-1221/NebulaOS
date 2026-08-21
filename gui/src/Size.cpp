// NebulaOS GUI - Size Implementation
// ===================================
//
// Implementation of 2D Size class

#include "../include/Size.h"
#include "../include/GuiTypes.h"
#include "../../lib/include/string.h"

namespace NebulaOS {
namespace GUI {

// -----------------------------------------------------------------------------
// ToString - Convert size to string representation
// -----------------------------------------------------------------------------
void Size::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    snprintf(buffer, size, "Size(%d, %d)", width, height);
}

} // namespace GUI
} // namespace NebulaOS
