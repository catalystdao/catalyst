forge compile

./deployCatalyst.canto.sh &
scroll=$!
./deployCatalyst.cronos.sh &
canto=$!
./deployCatalyst.scroll.sh &
cronos=$!
wait $scroll $canto $cronos

# Set the connections

set -o allexport
source .canto.env
set +o allexport

forge script DeployVaults --sig "setConnections()" --fork-url=$RPC_URL --broadcast &
scroll=$!

set -o allexport
source .cronos.env
set +o allexport

forge script DeployVaults --sig "setConnections()" --fork-url=$RPC_URL --broadcast &
canto=$!

set -o allexport
source .scroll.env
set +o allexport

forge script DeployVaults --sig "setConnections()" --fork-url=$RPC_URL --broadcast &
cronos=$!


wait $scroll $canto $cronos