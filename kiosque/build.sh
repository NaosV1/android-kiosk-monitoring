set -e
export ANDROID_DATA=/data ANDROID_ROOT=/system
JDK=$(ls -d $PREFIX/lib/jvm/java-21-openjdk* | head -1)
SRC=~/kiosque-src
B=$TMPDIR/kbuild
rm -rf "$B"; mkdir -p "$B/stubs-src" "$B/stubs" "$B/src" "$B/classes"

# Secrets kept in Termux home: control token + signing key password
[ -s ~/.kiosk-token ] || head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n' > ~/.kiosk-token
[ -s ~/.kiosk-keypass ] || head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n' > ~/.kiosk-keypass
chmod 600 ~/.kiosk-token ~/.kiosk-keypass
TOKEN=$(cat ~/.kiosk-token); KP=$(cat ~/.kiosk-keypass)

echo "== stubs"
bash "$SRC/gen-stubs.sh" "$B/stubs-src"
javac --release 8 -nowarn -d "$B/stubs" $(find "$B/stubs-src" -name '*.java') 2>&1 | grep -v 'obsolete\|warning' || true
(cd "$B/stubs" && jar cf ../stubs.jar .)

echo "== javac"
cp -r "$SRC/src/." "$B/src/"
sed -i "s/@TOKEN@/$TOKEN/" "$B/src/fr/tablette/kiosque/Control.java"
javac --release 8 -nowarn -cp "$B/stubs.jar" -d "$B/classes" $(find "$B/src" -name '*.java') 2>&1 | grep -v 'obsolete\|warning' || true
ls "$B/classes/fr/tablette/kiosque/"

echo "== d8"
d8 --release --min-api 28 --lib "$JDK" --lib "$B/stubs.jar" --output "$B" $(find "$B/classes" -name '*.class') 2>&1 | grep -v '^Warning' || true
ls -la "$B/classes.dex"

echo "== aapt"
aapt package -f -M "$SRC/AndroidManifest.xml" -S "$SRC/res" -I /system/framework/framework-res.apk -F "$B/unsigned.apk"
(cd "$B" && aapt add unsigned.apk classes.dex >/dev/null)
zipalign -f -p 4 "$B/unsigned.apk" "$B/aligned.apk"

echo "== sign"
[ -f ~/.kiosk.keystore ] || keytool -genkeypair -keystore ~/.kiosk.keystore -alias kiosque -keyalg RSA -keysize 2048 \
    -validity 10000 -storepass "$KP" -keypass "$KP" -dname "CN=Kiosque tablette" >/dev/null 2>&1
apksigner sign --ks ~/.kiosk.keystore --ks-key-alias kiosque --ks-pass "pass:$KP" --key-pass "pass:$KP" \
    --out ~/kiosque.apk "$B/aligned.apk"
apksigner verify ~/kiosque.apk && echo "signature OK"
aapt dump badging ~/kiosque.apk | grep -E "^package|testOnly|sdkVersion|targetSdk"
ls -la ~/kiosque.apk
