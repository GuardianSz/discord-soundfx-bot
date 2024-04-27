mysql -D soundfx -N -e "SELECT name, hex(src) FROM sounds" > out
split --additional-suffix=.row -l 1 out
for filename in *.row; do
    name=`grep -oP '^(.+)(?=\t)' $filename`
    col=`awk -F '\t' '{print $2}' "$filename"`
    echo $col > "$filename.hex"
    xxd -r -p "$filename.hex" "$name.opus"
done
