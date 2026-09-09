
#  Asteroids in Ada

## GCC GNAT

Install GNAT from a repository, use Alire to install a toolchain, or download and unpack a GNAT release from:

https://github.com/alire-project/GNAT-FSF-builds/releases

then add the resulting `bin` directory to your system's `$PATH`.

## Compile

```sh
gcc -c -fdump-ada-spec raylib/include/raylib.h
gnatmake asteroids -largs -L raylib/lib -lraylib
./asteroids
```

or edit the `RAYLIBPATH` in the Makefile and

```sh
make run
```

## Windows

Use a shell like [busybox for Windows](https://frippery.org/busybox/) (which includes `make`). The Makefile handles downloading and unpacking the windows raylib 6.0 release from github.

```sh
C:\asteroids> busybox.exe sh
C:/asteroids $ make run
```


