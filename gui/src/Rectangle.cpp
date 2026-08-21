// NebulaOS GUI - Rectangle Implementation
// =======================================
//
// Implementation of Rectangle class

#include "../include/Rectangle.h"
#include "../include/GuiTypes.h"
#include "../../lib/include/string.h"
#include "../../kernel/common/include/nebula.h"

namespace NebulaOS {
namespace GUI {

// -----------------------------------------------------------------------------
// ToString - Convert rectangle to string representation
// -----------------------------------------------------------------------------
void Rectangle::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    snprintf(buffer, size, "Rectangle(%d, %d, %d, %d)", x, y, width, height);
}

} // namespace GUI
} // namespace NebulaOS
