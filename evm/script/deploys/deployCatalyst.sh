./deployCatalyst.canto.sh &
scroll=$!
./deployCatalyst.cronos.sh &
canto=$!
./deployCatalyst.scroll.sh &
cronos=$!
wait $scroll $canto $cronos