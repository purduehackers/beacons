window.onkeydown = handleKeyDown;
window.onclick = handleClick;
//beacons have: 
/*{
	id: int
	owner?: int,
	title: str
	desc: str
	baseColor: hex (str)
}*/

//history items have:
/*{
	user: userID (int)
	title: str
	desc: str
	color: hex (str)
	date: unix time (int)
}*/

let localBeacons = [];
let localHistory = [];
var localUserID = 2;

var animation;

var unpairHeading = `[~Unpaired Beacon~]`;
var colorsList = [];
var colorSelected = `#000`;

var charLimitTitle = 128;
var charLimitDesc = 1024;

var openingFlag = false;

var searchBeaconID = -1;
var searchBeaconColor = "#000";
var searchBeaconFreq = 0;
var searchBeaconT0 = 0;

var shadowDiv = document.createElement("div");

function objectify(string) {
	shadowDiv.innerHTML = string;
	return shadowDiv.children[0];
}

function initColors() {
	for (var v=33; v<100; v+=33) {
		for (var s=40; s<100; s+=59) {
			for (var h=0; h<360; h+=30) {
				colorsList.push(HSVtoRGB({
					h: h,
					s: s,
					v: v / 100
				}));
			}
			colorsList.push(`BREAKER`);
		}
	}
	colorsList.push(`#FFFFFF`);

	var frozen = JSON.parse(JSON.stringify(colorsList));

	while (colorsList.length > 0) {
		var c = colorsList[0];
		popup_colors.appendChild(objectify((c != `BREAKER`) ? 
			`<button class="color-elem" onclick="selectColor('${c}')" style="background-color: ${c}"> </button>` : 
			`<br>`
		));
		colorsList.splice(0, 1);
	}

	colorsList = frozen;
}
initColors();

function selectColor(color) {
	colorSelected = color;
	popup_heading.style = `color: ${color}`;
}



//just do local time. thanks javascript
function time(unixTimeStamp) {
	var obj = new Date(unixTimeStamp);
	return (""+obj.getHours()).padStart(2, "0") + ":" + (""+obj.getMinutes()).padStart(2, "0");
}

function HSVtoRGB(hsvObj) {
	if (hsvObj.h < 0) {
		hsvObj.h += 360;
	}
	//I don't understand most of this but it appears to work
	var compound = hsvObj.v * (hsvObj.s / 100);
	var x = compound * (1 - Math.abs(((hsvObj.h / 60) % 2) - 1));
	var mystery = hsvObj.v - compound;
	var RGB = [0, 0, 0];

	switch(Math.floor(hsvObj.h / 60)) {
		case 0:
			RGB = [compound, x, 0];
			break;
		case 1:
			RGB = [x, compound, 0];
			break;
		case 2:
			RGB = [0, compound, x];
			break;
		case 3:
			RGB = [0, x, compound];
			break;
		case 4:
			RGB = [x, 0, compound];
			break;
		case 5:
			RGB = [compound, 0, x];
			break;
	}

	RGB = [
		Math.floor((RGB[0] + mystery) * 255), 
		Math.floor((RGB[1] + mystery) * 255), 
		Math.floor((RGB[2] + mystery) * 255)
	];
	//turn into hex representation
	RGB = RGB.map(x => Math.floor(x).toString(16).padStart(2, "0"));
	return `#` + RGB.join(``);
}





//API functions
function addBeacon(beaconObj) {
	//check if the object is already in the beacons list
	var bObj = getBeacon(beaconObj.id);

	if (bObj) {
		console.log(`${beaconObj.id} is already in array, modifying instead`);
		//if so, update info
		Object.keys(beaconObj).forEach(k => {
			bObj[k] = beaconObj[k];
		});

		//refresh the document
		doc_removeBeacon(bObj.id);
		doc_addBeacon(bObj.id);
		return;
	}

	//if it's not, add it
	localBeacons.push(beaconObj);
	doc_addBeacon(beaconObj);
}

function removeBeacon(beaconID) {
	doc_removeBeacon(beaconID);
	var bObj = getBeacon(beaconID);
	if (bObj) {
		localBeacons.splice(localBeacons.indexOf(bObj), 1);
	} else {
		console.error(`Cannot remove beacon with ID ${beaconID}`);
	}
}

//from the server: a specified beacon wants to pair. 
//If this works, and after the user enters in the details, the pairSuccess function is called.
function wantsToPair(beaconID) {
	console.log(`prepping`, beaconID);
	//not actually sure what this should do or when the server would send this.

	popup_heading.innerHTML = `your project name...`;
	popup_heading.contentEditable = true;
	popup_desc.innerHTML = `your description...`;
	popup_desc.contentEditable = true;

	popup_colors.style = `display: inline-block`;

	popup_lower.innerHTML = `Confirm`;
	popup_lower.onclick = () => {
		pairSuccess(beaconID);
	}
}

//sends message to the server that pairing with a specified beacon was a success.
//Sends a specified title, description, and base color
function pairSuccess(beaconID) {
	var goalObj = {
		id: beaconID,
		owner: localUserID,
		title: popup_heading.innerHTML,
		desc: popup_desc.innerHTML,
		baseColor: colorSelected
	};
	
	//add to local zone
	removeBeacon(beaconID);
	addBeacon(goalObj);
	closePopup();
}

//from the server: recieve one session worth of information (a session is a previous beacon's pairing information)
function recieveSession(sessionObj) {
	localHistory.push(sessionObj);
}

//to the server: set the specified beacon into search mode
function startSearch(beaconID) {
	
	//pretend we're the server right now
	searchRecieved(beaconID, colorsList[Math.floor(Math.random() * colorsList.length)], Math.random() * 1.2 + 0.5, (Date.now() / 1000) - 2);

	//ok we're the client again
	search_space.style = `display: inline-block`;
	popup_space.style = `display: none`;
}

//to the server: take the specified beacon out of search mode - CAVEAT FOR THE SERVER: only do this if nobody else is also searching
function endSearch(beaconID) {

}

//from the server: beacon #ID is in search mode with these parameters.
function searchRecieved(beaconID, color, frequency, startTime) {
	search_lower.onclick = () => {
		searchStop(beaconID);
	}
	[searchBeaconID, searchBeaconColor, searchBeaconFreq, searchBeaconT0] = arguments;
	search_baseBar.style = `background-color: ${getBeacon(beaconID).baseColor};`;
	console.log(...arguments);
	animation = window.requestAnimationFrame(searchUpdate);
}

function searchUpdate() {
	var t = ((Date.now() / 1000) - searchBeaconT0) / searchBeaconFreq
	var opacity = Math.abs(Math.sin(Math.PI * (t % 1)));

	search_pulseBar.style = `background-color: ${searchBeaconColor}; opacity: ${opacity};`;

	animation = window.requestAnimationFrame(searchUpdate);
}

//to the server: says that this client is no longer searching for beacon #ID. 
//Should take the beacon out of search mode if this client is the only one searching for it
function searchStop(beaconID) {
	endSearch(beaconID);
	window.cancelAnimationFrame(animation);
	closePopup();
}




function addHistory(historyObj) {
	//TODO: do some duplicate checking here, probably use the same policy as in the beacon adding
	localHistory.push(historyObj);
	doc_addHistory(historyObj);
}








//non-api functions

//gets a beacon from the list of existing beacons using its ID number
function getBeacon(id) {
	for (var a=0; a<localBeacons.length; a++) {
		if (localBeacons[a].id == id) {
			return localBeacons[a];
		}
	}
	return undefined;
}

function openPopup(beaconID) {
	closePopup();
	openingFlag = true;
	var bObj = getBeacon(beaconID);
	var paired = (bObj.title != undefined);
	popup_space.style = `display: inline-block`;
	popup_heading.innerHTML = paired ? bObj.title : unpairHeading;
	popup_desc.innerHTML = paired ? bObj.desc : `Beacon ID ${beaconID} is currently unpaired. If you are logged in and so desire, you can pair to it.`;

	//if unpaired: should have pair button, if paired: should have search button
	//surely this is the best coding practice
	var [text, func] = paired ? 
		[`Ping`, startSearch] : 
		[`Pair`, wantsToPair];

	console.log(func);
	
	popup_lower.innerHTML = text;
	//stupid
	popup_lower.onclick = () => {
		func(beaconID);
	};

	//brightness 90% so white isn't white
	popup_heading.style = `color: ${paired ? bObj.baseColor : "#000"}; brightness: 90%`;
	popup_colors.style = `display: none`;
}

//saveguard against things that aren't supposed to happen outside of a popup context
function closePopup() {
	popup_space.style = `display: none`;
	popup_colors.style = `display: none`;
	search_space.style = `display: none`;
	if (animation) {
		window.cancelAnimationFrame(animation);
	}

	popup_heading.contentEditable = false;
	popup_desc.contentEditable = false;
}


//
function doc_addBeacon(beaconObj) {
	var [id, baseColor, title, desc] = [beaconObj.id, beaconObj.baseColor, beaconObj.title, beaconObj.desc];
	//paired case
	var toPush = objectify(`
	<div id="beacon-box-${id}" class="beacon-box" onclick="openPopup(${id})">
		<h2 id="beacon-box-${id}-title" class="beacon-title" style="color: ${baseColor ?? `#000`};">${title ?? unpairHeading}</h2>
		<p id="beacon-box-${id}-desc" class="beacon-desc">${desc ?? (`ID: `+id)}</p>
	</div>`);
	main_space.appendChild(toPush);

	//put all nodes in alphabetical order based on title
	var sorted = [...main_space.children].sort((a, b) => {
		return (a.children[0].innerHTML > b.children[0].innerHTML) ? 1 : -1;
	});
	console.log(`sorting!`);
	//I have to do this instead of just setting children = sorted because children is an HTMLCollection, not an array
	sorted.forEach(node => main_space.appendChild(node));
}

function doc_removeBeacon(id) {
	var element = document.getElementById(`beacon-box-${id}`);
	element.remove();
}

//history boxes are like beacon boxes except slightly different. 
// They contain a bit more information because you won't be clicking on them to get a more detailed view
function doc_addHistory(historyObj) {
	var id = historyObj.id;
	history_space.appendChild(objectify(`
	<div id="history-box-${id}" class="history-box" onclick="openPopup(${id})">
		<h2 id="beacon-box-${id}-title" class="history-title" style="color: ${historyObj.baseColor};">${historyObj.date}: ${historyObj.title}</h2>
		<p id="beacon-box-${id}-desc" class="history-desc">${historyObj.desc}</p>
	</div>`));
}

function doc_removeHistory(id) {
	var element = document.getElementById(`beacon-box-${id}`);
	element.remove();
}

//preps the viewable history for a set userID
function doc_prepHistory() {
	localHistory.forEach(h => {
		//figure out if it should be visible or not
		var isVisible = (h.user == localUserID);
		var elem = document.getElementById(`history-box-${h.id}`);
		elem.style = `display: ${isVisible ? `inline-block` : `none`}`;
	});
}

function toggleToHistory() {
	doc_prepHistory();
	button_history.onclick = toggleFromHistory;
	main_space.style = `display: none;`;
	history_space.style = `display: inline-block;`;
	button_history.innerHTML = `View Active Beacons`;
}

function toggleFromHistory() {
	button_history.onclick = toggleToHistory;
	main_space.style = `display: grid;`;
	history_space.style = `display: none;`;
	button_history.innerHTML = `View Project History`;
}



function setup() {
	//move the svg into the actual svg area proper
	var svgDat = bechtel_map.getSVGDocument().documentElement;
	bechtel_map.remove();
	bege.appendChild(svgDat);
	handleResize();

	beacons.forEach(b => {
		doc_addBeacon(b.id);
	});
}

function handleResize() {
	//resize workspace to be slightly larger than the actual document size. It's ok because we're disabling scroll bars so they'll never know
	var w =  window.innerWidth;
	var h = window.innerHeight;
	base.setAttribute("width", w);
	base.setAttribute("height", h);
}

function handleKeyDown(e) {
	switch (e.code) {
		//pull down the pop-up panel
		case "Escape":
			closePopup();
			break;
	}
}

function handleClick(e) {
	var list = [...document.querySelectorAll(":hover")];
	//if the popup space is open, and clicked off of anything, close it
	//has to be <4 instead of ==1 because the main beacon margin makes 3
	if (!openingFlag && !list.includes(popup_space) && !list.includes(search_space)) {
		closePopup();
	}
	openingFlag = false;
}