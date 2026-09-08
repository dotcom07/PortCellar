/* Isolate Wine DirectDraw initialization without SC4 or SCGL. */
#define COBJMACROS
#include <windows.h>
#include <ddraw.h>
#include <stdio.h>

int main(void)
{
    IDirectDraw *dd = NULL;
    DDCAPS hw = {0}, sw = {0};
    hw.dwSize = sizeof(hw);
    sw.dwSize = sizeof(sw);
    fprintf(stderr, "probe: before DirectDrawCreate\n");
    HRESULT hr = DirectDrawCreate(NULL, &dd, NULL);
    fprintf(stderr, "probe: DirectDrawCreate=%08lx\n", (unsigned long)hr);
    if (FAILED(hr)) return 1;
    hr = IDirectDraw_GetCaps(dd, &hw, &sw);
    fprintf(stderr, "probe: GetCaps=%08lx\n", (unsigned long)hr);
    IDirectDraw_Release(dd);
    return FAILED(hr) ? 2 : 0;
}
