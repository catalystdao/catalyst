#!/bin/bash

# Helper script to wait for IBC channel to be OPEN on channel-0

testDir=$(ps aux | grep polymerased | head -1 | awk '{print $(NF-1)}')
ready=$(polymerased --home ${testDir} --node http://127.0.0.1:11000 --chain-id polymerase --output json q ibc channel channels | jq '.channels[] | select(.channel_id == "channel-0" and .state == "STATE_OPEN") | [.] | length')
echo -n "Waiting for IBC channels to be in OPEN state..."
while [ "${ready}x" != '1x' ]; do
  ready=$(polymerased --home ${testDir} --node http://127.0.0.1:11000 --chain-id polymerase --output json q ibc channel channels | jq '.channels[] | select(.channel_id == "channel-0" and .state == "STATE_OPEN") | [.] | length')
  sleep 2
  echo -n '.'
done
echo 'done!'
