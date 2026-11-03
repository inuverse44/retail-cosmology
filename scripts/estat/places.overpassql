[out:csv(::lat,::lon;false;",")][timeout:600];
area["ISO3166-1"="JP"][admin_level=2]->.jp;
node(area.jp)["place"~"^(city|town|village|hamlet|suburb|quarter|neighbourhood|isolated_dwelling|locality)$"];
out;
