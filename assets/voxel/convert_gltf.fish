#!/usr/bin/fish
set --local options 'i/input=' 'o/output=' 'p/prefix='
argparse $options -- $argv
set -l SED_CMD $(string replace  "\d" "[[:digit:]]" $_flag_prefix );

for OBJ_PATH in (ls obj/$_flag_input/*.obj); 
    set -l FILE $(path basename $OBJ_PATH);
    set -l SOURCE_PATH $(path normalize "obj/$_flag_input/$FILE");
    set -l DEST_FILE $(path change-extension '.gltf' $FILE);
    set -l DEST_PATH $(path normalize "gltf/$_flag_output/$DEST_FILE" | sed "s/$SED_CMD//");
    echo $DEST_PATH;
    npx obj2gltf -i $SOURCE_PATH -o $DEST_PATH;
end
