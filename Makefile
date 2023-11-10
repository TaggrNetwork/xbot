deploy:
	dfx --identity prod deploy --network ic

logs:
	dfx --identity prod canister --network ic call --query xbot info '("logs")'

stats:
	dfx --identity prod canister --network ic call --query xbot info '("stats")'
