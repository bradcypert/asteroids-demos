BIN = asteroids
RAYLIBPATH = ./raylib-6.0_win64_mingw-w64
FLAGS = -O3
LDFLAGS = -s -L $(RAYLIBPATH)/lib

ifeq ($(OS),Windows_NT)
# for a static build without raylib.dll: -lraylib -lgdi32 -lwinmm
	LDFLAGS += -lraylibdll
else
	LDFLAGS += -lraylib
endif

.PHONY: all run clean distclean

all: $(BIN)

# Compile with gnatmake

raylib_h.ads: $(RAYLIBPATH)
	gcc -c -fdump-ada-spec $(RAYLIBPATH)/include/raylib.h

$(BIN): asteroids.adb raylib_h.ads
	gnatmake $< -o $(BIN) $(FLAGS) -largs $(LDFLAGS)

run: $(BIN)
	ifeq ($(OS),Windows_NT)
		cp $(RAYLIBPATH)/lib/raylib.dll .
	endif
	./$(BIN)

# Clean
clean:
	rm -f *.o $(BIN) $(BIN).exe *.ali
	rm -f *.ads

distclean: clean
	rm -rf raylib-6.0_win64_mingw-w64
	rm -f raylib-6.0_win64_mingw-w64.zip
	rm -f raylib.dll

# Windows, download raylib from github releases
ifeq ($(OS),Windows_NT)

./raylib-6.0_win64_mingw-w64: raylib-6.0_win64_mingw-w64.zip
	tar.exe xf raylib-6.0_win64_mingw-w64.zip

raylib-6.0_win64_mingw-w64.zip:
	curl -LO https://github.com/raysan5/raylib/releases/download/6.0/raylib-6.0_win64_mingw-w64.zip

endif
