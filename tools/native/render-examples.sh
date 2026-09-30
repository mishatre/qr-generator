#!/usr/bin/env bash
#
# Render a set of sample codes into build/examples/, with an index page to look at
# them in a browser.
#
# Why this exists next to the test suite: the tests decode what they render, which
# answers "can a scanner read this". They cannot answer "does this look right", and
# those are different questions - a plate that swallows the timing pattern still
# decodes from some angles while looking broken to a person. This is the visual
# check to run before believing a layout, colour or plate-size change.
#
# Usage:
#   tools/native/render-examples.sh [output-dir]      # default: build/examples
#
# The images are generated, never committed: build/ is ignored.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${1:-$REPO_ROOT/build/examples}"

cd "$REPO_ROOT/rust"
echo "==> building the render example (release)"
cargo build --release --example render -p qr_generator_addin >/dev/null
RENDER="$REPO_ROOT/rust/target/release/examples/render"

# The record a live service issues, with the check code it computed for it.
RECORD='2|2000020302|0085F0226|01022031|5RCD6VPEAX4R'
CHECK='AX4R'
URL='https://example.com/orders/2026/0001?source=qr&station=14'

mkdir -p "$OUT"
INDEX="$OUT/index.html"

filesize() {
	if stat -f%z "$1" >/dev/null 2>&1; then stat -f%z "$1"; else stat -c%s "$1"; fi
}

# The page is written as the images are rendered, so a failed case leaves a
# readable page behind instead of nothing.
cat >"$INDEX" <<'HTML'
<!DOCTYPE html>
<html lang="ru">
<head>
<meta charset="utf-8">
<title>Примеры генерации QR-кодов</title>
<style>
 body { font: 14px/1.5 -apple-system, "Segoe UI", sans-serif; margin: 24px; max-width: 1100px; }
 h1 { font-size: 20px; }
 .case { display: flex; gap: 20px; padding: 16px 0; border-top: 1px solid #ddd; }
 .shot { flex: 0 0 300px; }
 /* Шахматка под картинкой: без нее прозрачный фон не отличить от белого. */
 .shot img { width: 300px; image-rendering: pixelated;
   background: repeating-conic-gradient(#e8e8e8 0 25%, #fbfbfb 0 50%) 0 0/16px 16px; }
 .meta { flex: 1 1 auto; min-width: 0; }
 .meta h2 { font-size: 15px; margin: 0 0 4px; font-family: ui-monospace, monospace; }
 .meta p { margin: 0 0 8px; }
 pre { background: #f6f6f6; padding: 8px 10px; overflow-x: auto; margin: 0;
   font: 12px/1.45 ui-monospace, monospace; }
 .size { color: #666; }
</style>
</head>
<body>
<h1>Примеры генерации QR-кодов</h1>
<p>Сгенерировано <code>tools/native/render-examples.sh</code> и не хранится в репозитории:
каталог <code>build/</code> в историю не попадает. Под каждой картинкой - то, с чем она
сгенерирована, и на что смотреть. Прозрачный фон показан шахматкой.</p>
HTML

printf '%-18s %8s  %s\n' case bytes note

render() {
	local name="$1" data="$2" label="$3" options="$4" note="$5"
	"$RENDER" "$data" "$label" "$OUT/$name.png" "$options" >/dev/null
	local size
	size="$(filesize "$OUT/$name.png")"
	printf '%-18s %8s  %s\n' "$name" "$size" "$note"

	{
		printf '  <div class="case">\n    <div class="shot"><img src="%s.png" alt="%s"></div>\n' "$name" "$name"
		printf '    <div class="meta">\n      <h2>%s</h2>\n      <p>%s</p>\n' "$name" "$note"
		printf '      <pre>данные: %s\nподпись: %s\nнастройки: %s</pre>\n' \
			"$data" "${label:-(нет)}" "${options:-(по умолчанию)}"
		printf '      <p class="size">%s байт</p>\n    </div>\n  </div>\n' "$size"
	} >>"$INDEX"
}

render record-default "$RECORD" "$CHECK" "" \
	"Запись рабочего сервиса с проверочным кодом. Настройки по умолчанию: 600 пикселей, уровень H, подложка 0.3 стороны, фон прозрачный."
render record-no-label "$RECORD" "" "" \
	"Те же данные без подписи: сравнить с первым примером - подпись меняет картинку, но не данные в коде."
render record-printable "$RECORD" "$CHECK" '{"background": "#ffffff"}' \
	"Тот же код на белом фоне: вариант для печати."
render record-1024 "$RECORD" "$CHECK" '{"size": 1024}' \
	"1024 пикселя на сторону: для печати, где нужен запас по разрешению."
render label-0.25 "$RECORD" "$CHECK" '{"labelSize": 0.25}' \
	"Подложка 0.25 стороны: читается и на символе с малым числом модулей."
render label-0.35 "$RECORD" "$CHECK" '{"labelSize": 0.35}' \
	"Подложка 0.35: измеренная граница для символа в 25 модулей."
render label-0.4 "$RECORD" "$CHECK" '{"labelSize": 0.4}' \
	"Подложка 0.4: на символе в 25 модулей код перестает читаться - подложка закрывает больше модулей, чем восстанавливает коррекция."
render label-transparent "$RECORD" "$CHECK" '{"labelBackground": "transparent"}' \
	"Прозрачная подложка: модули под подписью удалены, на их месте отверстие, а текст подписи остается. Видно на шахматке."
render quiet-zone-4 "$RECORD" "$CHECK" '{"quietZone": 4, "labelSize": 0.25}' \
	"Зона покоя 4 модуля: нужна, когда код печатается вплотную к краю."
render ec-l "$RECORD" "$CHECK" '{"errorCorrectionLevel": "L"}' \
	"Уровень коррекции L (7 процентов) при той же подписи: видно, почему умолчание - H."
render ec-q "$RECORD" "$CHECK" '{"errorCorrectionLevel": "Q"}' \
	"Уровень Q (25 процентов) - между L и умолчанием H."
render colors "$RECORD" "$CHECK" '{"foreground": "#0b3d91", "background": "#ffffff", "labelColor": "#0b3d91", "labelBackground": "#f2f2f2", "labelSize": 0.28}' \
	"Свои цвета: модули, фон, текст подписи и подложка."
render long-data "$URL" "ORDER 0042" '{"labelRows": 2}' \
	"Длинная ссылка и подпись в две строки (labelRows = 2)."
render long-label "$RECORD" "5RCD6VPEAX4R" "" \
	"Длинный текст подписи при автоматическом подборе числа строк."

cat >>"$INDEX" <<'HTML'
</body>
</html>
HTML

echo
echo "examples: $OUT"
echo "index:    $INDEX"
echo "open it:  open \"$INDEX\""
