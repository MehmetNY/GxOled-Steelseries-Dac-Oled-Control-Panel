package bitmap

import "image"
import "image/color"

func DrawBorder(img *image.Gray, colorValue uint8) {
	bounds := img.Bounds()
	if bounds.Empty() {
		return
	}
	w, h := bounds.Dx(), bounds.Dy()
	col := color.Gray{Y: colorValue}
	for x := 0; x < w; x++ {
		img.SetGray(x, 0, col)
		img.SetGray(x, h-1, col)
	}
	for y := 0; y < h; y++ {
		img.SetGray(0, y, col)
		img.SetGray(w-1, y, col)
	}
}
