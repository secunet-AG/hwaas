#!/usr/bin/env bash
set -eoux pipefail

echo "Starting usb test"
# shellcheck disable=SC1091
. raspi_utils.sh

BOOT_IMAGE=$1
BOOT_IMAGE_FILENAME=$(basename -- "$BOOT_IMAGE")
TMP_PATH="/tmp"
IMAGE_BASE_PATH="/run/remote-usb"
EMPTY_IMAGE="/tmp/usb_test_write_image"
EMPTY_IMAGE_FILENAME=$(basename -- "$EMPTY_IMAGE")
TS_HOST=$2
POWER_PORT=8081
SERIAL_PORT=8082
USB_PORT=8083

# Clean-up preparations

function _cleanup() {
  function remove_file() {
    ssh "$TS_HOST" "sudo rm -f $TMP_PATH/$1"
    ssh "$TS_HOST" "sudo rm -f $IMAGE_BASE_PATH/$1"
  }
  remove_file "$BOOT_IMAGE_FILENAME"
  remove_file "$EMPTY_IMAGE_FILENAME"
  http -I --quiet --check-status POST "$TS_HOST:$SERIAL_PORT/serial/reset"
  http -I --quiet --check-status POST "$TS_HOST:$POWER_PORT/power/reset"
  http -I --quiet --check-status POST "$TS_HOST:$USB_PORT/usb/reset"
}

trap _cleanup EXIT

_cleanup

# Prepare (boot) images

echo "create 1MB empty image"
dd bs=1M count=1 if=/dev/zero of="$EMPTY_IMAGE"

function prepare_image() {
  scp "$1" "$TS_HOST:$TMP_PATH/$2"
  ssh "$TS_HOST" "sudo chown root:root $TMP_PATH/$2"
  ssh "$TS_HOST" "sudo chmod 644 $TMP_PATH/$2" # -rw-r--r--
  ssh "$TS_HOST" "ls -lah $TMP_PATH/$2"
  ssh "$TS_HOST" "sudo mv $TMP_PATH/$2 $IMAGE_BASE_PATH/"
  ssh "$TS_HOST" "ls -lah $IMAGE_BASE_PATH/$2"
}

prepare_image "$BOOT_IMAGE" "$BOOT_IMAGE_FILENAME"
prepare_image "$EMPTY_IMAGE" "$EMPTY_IMAGE_FILENAME"

# Test if boot image works

echo "check machine power"
response=$(http --check-status GET "$TS_HOST:$POWER_PORT/power/custom")
echo "$response"
power=$(echo "$response" | jq -r '.state')
assertEqual "$power" "false"

echo "configure storage usb"
http --check-status PUT "$TS_HOST:$USB_PORT/usb" \
  <<<"[{ \"type\": \"storage\", \"luns\": [{\"path\": \"$IMAGE_BASE_PATH/$BOOT_IMAGE_FILENAME\", \"cdrom\": false, \"read_only\": false}] }]"

echo "check usb api"
image=$(http --check-status GET "$TS_HOST:$USB_PORT/usb" | jq -r '.[0].luns.[0].path')
assertEqual "$image" "$BOOT_IMAGE_FILENAME"

echo "enable machine power"
http --check-status PUT "$TS_HOST:$POWER_PORT/power/custom"

echo "check machine power after switch on"
power=$(http --check-status GET "$TS_HOST:$POWER_PORT/power/custom" | jq -r '.state')
assertEqual "$power" "true"

waitForCommandPrompt "$TS_HOST:$SERIAL_PORT" # if this returns successfully, then the boot was successful

# Test if pre-configured tty serial works

echo "execute whoami"
http --quiet --check-status POST "$TS_HOST:$SERIAL_PORT/serial/tty" \
  Content-Type:application/octet-stream <<<"whoami"

waitForCommandPrompt "$TS_HOST:$SERIAL_PORT"

echo "verify whoami"
username=$(http -I --check-status GET "$TS_HOST:$SERIAL_PORT/serial/tty" |
  clearAnsi |
  tr -d '\0\r' |
  grep -a -A1 "whoami" |
  tail -n1)
assertEqual "$username" "nixos"

# Test if clearing pre-configured tty serial works

function clear_serial() {
  echo "clear serial"
  http -I --quiet --check-status DELETE "$TS_HOST:$SERIAL_PORT/serial/tty"
  serial=$(http -I --check-status GET "$TS_HOST:$SERIAL_PORT/serial/tty")
  assertEqual "$serial" ""
}

clear_serial

# Test if providing no functions works

http --check-status PUT "$TS_HOST:$USB_PORT/usb" <<<"[]"

# Test if power off works

function power_off() {
  echo "disable machine power"
  http -I --check-status DELETE "$TS_HOST:$POWER_PORT/power/custom"

  echo "check machine power after switch off"
  power=$(http --check-status GET "$TS_HOST:$POWER_PORT/power/custom" | jq -r '.state')
  assertEqual "$power" "false"
}

power_off

# Test if deactivating USB config works

function deconfigure_usb() {
  echo "deconfigure usb"
  http -I --check-status DELETE "$TS_HOST:$USB_PORT/usb"

  echo "check usb functions after deconfiguration"
  usb=$(http --check-status GET "$TS_HOST:$USB_PORT/usb")
  assertEqual "$usb" "[]"
}

deconfigure_usb

# Test if providing multiple images works

function check_usb_and_boot() {
  echo "check usb api"
  result=$(http --check-status GET "$TS_HOST:$USB_PORT/usb" | jq -r "$1")
  assertEqual "$result" "$2"

  echo "enable machine power"
  http --check-status PUT "$TS_HOST:$POWER_PORT/power/custom"

  echo "check machine power after switch on"
  power=$(http --check-status GET "$TS_HOST:$POWER_PORT/power/custom" | jq -r '.state')
  assertEqual "$power" "true"

  waitForCommandPrompt "$TS_HOST:$SERIAL_PORT"
}

echo "specify mass storage device with multiple storages"
http --quiet --check-status PUT "$TS_HOST:$USB_PORT/usb" \
  <<<"[{ \"type\": \"storage\", \"luns\": [
          {\"path\": \"$IMAGE_BASE_PATH/$BOOT_IMAGE_FILENAME\", \"cdrom\": false, \"read_only\": false},
          {\"path\": \"$IMAGE_BASE_PATH/$EMPTY_IMAGE_FILENAME\", \"cdrom\": false, \"read_only\": true}
          ]
        }]"

check_usb_and_boot '.[0].luns | .[] | .path' "$BOOT_IMAGE_FILENAME
$EMPTY_IMAGE_FILENAME"

echo "check that 2nd storage is present under /dev/sdc"
http --quiet --check-status POST "$TS_HOST:$SERIAL_PORT/serial/tty" \
  Content-Type:application/octet-stream <<<'test -e /dev/sdc; echo $?'
sleep .5
returnCode=$(http -I --check-status GET "$TS_HOST:$SERIAL_PORT/serial/tty" |
  tr -d '\0\r' |
  tail -3 |
  head -1)
assertContains "$returnCode" "0"

clear_serial
power_off

# Test if HID support works

echo "specify HID device: keyboard"
http --quiet --check-status PUT "$TS_HOST:$USB_PORT/usb" \
  <<<"[
    { \"type\": \"storage\", \"luns\": [{\"path\": \"$IMAGE_BASE_PATH/$BOOT_IMAGE_FILENAME\", \"cdrom\": false, \"read_only\": false}] },
    { \"type\": \"keyboard\" },
    { \"type\": \"mouse\" }
    ]"
http --check-status GET "$TS_HOST:$USB_PORT/usb"
check_usb_and_boot '.[] | .type' "storage
keyboard
mouse"

## Send keyboard commands in both possible ways

### As a String
http --quiet --check-status POST "$TS_HOST:$USB_PORT/usb/keyboard/text" \
  input="Hello " newline:=false
http --quiet --check-status POST "$TS_HOST:$USB_PORT/usb/keyboard/text" \
  input="World!" newline:=true

### As a report
#### send capital 'A'
http --quiet --check-status POST "$TS_HOST:$USB_PORT/usb/keyboard/report" \
  keys:='["A"]' modifier:=0 press:=true release:=true
#### send 'a' but make it capital via modifier
http --quiet --check-status POST "$TS_HOST:$USB_PORT/usb/keyboard/report" \
  keys:='["a"]' modifier:=2 press:=true release:=true
#### send 'a' but make it capital via pressing 'left-shift' at the same time
http --quiet --check-status POST "$TS_HOST:$USB_PORT/usb/keyboard/report" \
  keys:='["a", "left-shift"]' modifier:=0 press:=true release:=true
#### send 'a' after pressing caps-lock
http --quiet --check-status POST "$TS_HOST:$USB_PORT/usb/keyboard/report" \
  keys:='["caps-lock"]' modifier:=0 press:=true release:=true
http --quiet --check-status POST "$TS_HOST:$USB_PORT/usb/keyboard/report" \
  keys:='["a"]' modifier:=0 press:=true release:=true
#### send newline (must be send via /report, since /text would send '\' and 'n')
http --quiet --check-status POST "$TS_HOST:$USB_PORT/usb/keyboard/report" \
  keys:='["\n"]' modifier:=0 press:=true release:=true

#### send mouse report
http --quiet --check-status POST "$TS_HOST:$USB_PORT/usb/mouse" \
  buttons:='[3]' x:=10 y:=10 wheel:=0

# Test if dynamically configured tty serial works

# power_off

# echo "specify serial function"
# http --quiet --check-status PUT "$TS_HOST:$USB_PORT/usb" \
#   <<<"[
#     { \"type\": \"storage\", \"luns\": [{\"path\": \"$IMAGE_BASE_PATH/$BOOT_IMAGE_FILENAME\", \"cdrom\": false, \"read_only\": false}] },
#     { \"type\": \"serial\", \"serial_id\": \"dynamic\" }
#     ]"

# check_usb_and_boot '.[1].serial_id' "dynamic"

# TODO: remove debugging stop here
# This is helpful for manual debugging on the Raspberry Pi while the code above configures dynamic serial beforehand
# read -r -p "Press any key to continue... " -n1 -s

# # TODO: this fails with "Input/Output error", fix this and remove comment from test code
# echo "execute whoami"
# http --quiet --check-status POST "$TS_HOST:$USB_PORT/serial/dynamic" \
#   Content-Type:application/octet-stream <<<"whoami"

# waitForCommandPrompt "$TS_HOST:$SERIAL_PORT"

# # TODO: Does this work? Since we don't write anything we can't read anything...
# echo "verify whoami"
# username=$(http -I --check-status GET "$TS_HOST:$USB_PORT/serial/dynamic" |
#   clearAnsi |
#   tr -d '\0\r' |
#   grep -a -A1 "whoami" |
#   tail -n1)
# assertEqual "$username" "nixos"

# Done

echo "Successful usb test"
