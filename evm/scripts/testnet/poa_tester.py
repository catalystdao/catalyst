import argparse

from poa_router import PoARouter


def main():
    parser = argparse.ArgumentParser("proxy relayer")
    parser.add_argument("config_location", nargs='?', help="The path to the config location", type=str)
    args = parser.parse_args()
    config_location = "./scripts/deploy_config.json"
    if args.config_location:
        config_location = args.config_location
        
    relayer = PoARouter(config_name=config_location)
    print(relayer.backcheck())
    

if __name__ == "__main__":
    main()