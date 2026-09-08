#!/bin/bash
export DISPLAY=:0
export SLINT_FULLSCREEN=1
export RUST_LOG=info
export WOLT_TRACKING_URL=https://consumer-api.wolt.com/order-tracking-api/v1/details/tracking-code/track/
export FOOD_SERVER_URL="0.0.0.0:1337"

# Function to check network connection, regardless of interface (Wi-Fi or Ethernet)
check_network() {
    ping -c 1 -W 2 8.8.8.8 > /dev/null 2>&1
}

# Wait for network connection
while ! check_network; do
    echo "Waiting for network connection..."
    sleep 5
done

(/home/infoskjerm/bin/infoskjerm >> infoskjerm.log 2>&1) &