function url_domain(data) {
    var a = document.createElement('a');
    a.href = data;
    return a.hostname;
}

function linkify_headers() {
	// Do the stuff to the headers to linkify them

	$.each($('h2'), function() {
		$(this).addClass("anchor");
		$(this).append('&nbsp;<a class="tag" href="#' + $(this).attr('id') + '">¶</a>');
	});
	$('h2').hover(function() {
		$(this).children('.tag').css('display', 'inline');
	}, function() {
		$(this).children('.tag').css('display', 'none');
	});
}

function configure_navigation() {
	var navigation_items = $("#navbar-list > li");
	var windowpath = window.location.pathname;

	// Iterate over the list items and change the container of the active nav item to active
	$.each(navigation_items, function() {
		var elementlink = $(this).children($("a"))[0];
		var elementpath = elementlink.getAttribute("href");
		if (elementpath === windowpath) {
			$(this).addClass("active");
		}
	});
}

function set_anchor_height() {
	var navigation_height = $(".navbar").css("height");
	$(".anchor").css("padding-top", navigation_height);
	$(".anchor").css("margin-top", "-" + navigation_height);
}

// Run by default
linkify_headers();
configure_navigation();
set_anchor_height();
