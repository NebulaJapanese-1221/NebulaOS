// Build metadata for NebulaOS, embedded at link time.
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// See LICENCE for the full license text.

// These symbols are normally provided by the build system via
// compiler defines. When those are not set, fall back to static
// strings so the kernel always has something to report at boot.

#ifndef NEBULA_BUILD_DATE
#define NEBULA_BUILD_DATE "unknown"
#endif

#ifndef NEBULA_BUILD_SHA
#define NEBULA_BUILD_SHA "unknown"
#endif

const char git_build_date[] = NEBULA_BUILD_DATE;
const char git_build_sha[] = NEBULA_BUILD_SHA;