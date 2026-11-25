#!/usr/bin/env bash
# Runs every bundled engine the way PaperFlow does, inside the installed snap,
# on documents generated here. Catches a snap that builds but cannot convert
# (for example a LibreOffice that aborts while starting up).
#
# Usage (with the snap installed):
#   snap run --shell paperflow -c "bash $PWD/scripts/snap-smoke-test.sh"
set -uo pipefail
: "${SNAP:?run this inside the snap: snap run --shell paperflow -c 'bash scripts/snap-smoke-test.sh'}"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work" || exit 1
bin="$SNAP/usr/bin"
program="$SNAP/usr/lib/libreoffice/program"
export LC_ALL=C
gs_dir=$(find "$SNAP/usr/share/ghostscript" -mindepth 1 -maxdepth 1 -type d -name '[0-9]*' | sort | tail -1)
export GS_LIB="$gs_dir/Resource/Init:$gs_dir/lib:$gs_dir/Resource/Font:$SNAP/usr/share/ghostscript/fonts:$SNAP/usr/share/fonts/type1/urw-base35"
export TESSDATA_PREFIX="$SNAP/usr/share/tesseract-ocr/5/tessdata"

failures=0
check() { # check <description> <file that must exist and not be empty> [text it must contain]
  if [ ! -s "$2" ]; then
    echo "FAIL $1"
    sed 's/^/     /' log 2>/dev/null | tail -15
    failures=$((failures + 1))
  elif [ -n "${3:-}" ] && ! "$bin/pdftotext" "$2" - 2>/dev/null | grep -q "$3"; then
    echo "FAIL $1: \"$3\" not found in the result"
    failures=$((failures + 1))
  else
    echo "ok   $1"
  fi
}

# --- Fixtures ---------------------------------------------------------------
cat > report.fodt <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text"><office:body><office:text><text:h text:outline-level="1">Quarterly report</text:h><text:p>Revenue grew by twelve percent.</text:p></office:text></office:body></office:document>
EOF
cat > deck.fodp <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation"><office:body><office:presentation><draw:page draw:name="One"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="20cm" svg:height="3cm"><draw:text-box><text:p>First slide</text:p></draw:text-box></draw:frame></draw:page><draw:page draw:name="Two"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="20cm" svg:height="3cm"><draw:text-box><text:p>Second slide</text:p></draw:text-box></draw:frame></draw:page></office:presentation></office:body></office:document>
EOF
printf 'Item,Cost\nPaper,12\nInk,30\n' > budget.csv
printf '<!doctype html><html><body><h1>Meeting notes</h1><p>Ship <b>PaperFlow</b> on Friday.</p></body></html>' > notes.html

soffice() { # soffice <--convert-to value> <input> <outdir>
  local profile
  profile=$(mktemp -d)
  mkdir -p "$3"
  LD_LIBRARY_PATH="$program:${LD_LIBRARY_PATH:-}" SAL_USE_VCLPLUGIN=svp \
    "$program/soffice" --headless --invisible --nologo --norestore --nolockcheck --nodefault \
    "-env:UserInstallation=file://$profile" --convert-to "$1" --outdir "$3" "$2" > log 2>&1
  rm -rf "$profile"
}

# --- LibreOffice: Office formats to PDF --------------------------------------
# Microsoft formats are made by LibreOffice itself, then converted back.
soffice docx report.fodt .
soffice pptx deck.fodp .
soffice xlsx budget.csv .
check "LibreOffice writes .docx" report.docx
soffice pdf:writer_pdf_Export report.docx word
check "Word to PDF" word/report.pdf "Revenue grew"
soffice 'pdf:writer_pdf_Export:{"SelectPdfVersion":{"type":"long","value":"2"}}' report.docx pdfa
check "Word to PDF/A" pdfa/report.pdf "Quarterly report"
soffice pdf:impress_pdf_Export deck.pptx slides
check "PowerPoint to PDF" slides/deck.pdf "Second slide"
soffice pdf:calc_pdf_Export budget.xlsx sheet
check "Excel to PDF" sheet/budget.pdf "Paper"
soffice pdf:writer_web_pdf_Export notes.html html
check "HTML to PDF" html/notes.pdf "Meeting notes"

pdf=word/report.pdf
[ -s "$pdf" ] || { "$bin/gs" -q -sDEVICE=pdfwrite -o report.pdf -c '/Helvetica 20 selectfont 72 700 moveto (Quarterly report) show showpage' > log 2>&1; pdf=report.pdf; }

# --- Poppler, Ghostscript, qpdf ---------------------------------------------
"$bin/gs" -sDEVICE=pdfwrite -dCompatibilityLevel=1.6 -dNOPAUSE -dBATCH -dSAFER -dPDFSETTINGS=/ebook \
  -sOutputFile=compressed.pdf -- "$pdf" > log 2>&1
check "Compress (Ghostscript)" compressed.pdf "Quarterly report"
"$bin/pdftoppm" -png -r 72 -f 1 -l 1 -singlefile "$pdf" page > log 2>&1
check "PDF to PNG (pdftoppm)" page.png
"$bin/pdftotext" -enc UTF-8 -layout "$pdf" text.txt > log 2>&1
check "PDF to text (pdftotext)" text.txt
"$bin/pdftohtml" -xml -i -q -enc UTF-8 "$pdf" layout > log 2>&1
check "PDF to Markdown layout (pdftohtml)" layout.xml
printf -- '--encrypt\n--user-password=u\n--owner-password=o\n--bits=256\n--\n%s\nlocked.pdf\n' "$pdf" | "$bin/qpdf" @- > log 2>&1
check "Protect (qpdf)" locked.pdf
printf -- '--password=u\n--decrypt\n--\nlocked.pdf\nunlocked.pdf\n' | "$bin/qpdf" @- > log 2>&1
check "Unlock (qpdf)" unlocked.pdf "Quarterly report"

# --- A scanned page: extract images and OCR ---------------------------------
"$bin/gs" -q -sDEVICE=pdfimage24 -r150 -o scan.pdf \
  -c '/Helvetica-Bold 40 selectfont 72 650 moveto (SCANNED INVOICE) show showpage' > log 2>&1
"$bin/pdfimages" -all -p scan.pdf img > log 2>&1
check "Extract images (pdfimages)" "$(ls img-* 2>/dev/null | head -1)"
"$bin/pdftoppm" -r 150 -jpeg -singlefile scan.pdf ocr-page > log 2>&1
"$bin/tesseract" ocr-page.jpg ocr -l eng --dpi 150 pdf > log 2>&1
check "OCR (Tesseract)" ocr.pdf "INVOICE"

echo
if [ "$failures" -gt 0 ]; then
  echo "$failures check(s) failed"
  exit 1
fi
echo "all checks passed"
