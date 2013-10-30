// Do the stuff to the headers to linkify them
$.each($('h2'), function() {
	$(this).prepend('<a class="anchor" id="' + $(this).attr('id') + '-anchor"></a>');
    $(this).append('&nbsp;<a class="tag" href="#' + $(this).attr('id') + '-anchor">¶</a>');
});
$('h2').hover(function() {
    $(this).children('.tag').css('display', 'inline');
}, function() {
    $(this).children('.tag').css('display', 'none');
});
