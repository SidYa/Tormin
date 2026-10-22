cmake_minimum_required(VERSION 3.25)

if(WIN32)
  # Static CRT so TorminCore.dll needs no VC++ redistributable on the user's machine
  set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
  set(VCPKG_TARGET_TRIPLET "x64-windows-static" CACHE STRING "")
endif()

project(TorminCore LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_VISIBILITY_PRESET hidden)

# Windows: libtorrent from vcpkg (static). Linux: the distribution package (libtorrent-rasterbar-dev).
find_package(LibtorrentRasterbar CONFIG REQUIRED)

add_library(TorminCore SHARED tormin_core.cpp tormin_core.h)
target_link_libraries(TorminCore PRIVATE LibtorrentRasterbar::torrent-rasterbar)

if(MSVC)
  target_compile_definitions(TorminCore PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX _WIN32_WINNT=0x0A00)
  target_compile_options(TorminCore PRIVATE /W3 /utf-8 /bigobj)
else()
  target_compile_options(TorminCore PRIVATE -Wall)
  # Look for libtorrent next to libTorminCore.so first: the AppImage ships both together
  set_target_properties(TorminCore PROPERTIES BUILD_RPATH "\$ORIGIN" INSTALL_RPATH "\$ORIGIN")
endif()
