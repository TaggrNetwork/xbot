deploy:
	dfx --identity prod deploy --network ic

logs:
	dfx canister --network ic call --query xbot info '("logs")'

stats:
	dfx canister --network ic call --query xbot info '("stats")'

status:
	dfx --identity prod canister --network ic status xbot
