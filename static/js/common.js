// Do the stuff to the headers to linkify them
$.each($('h2'), function() {
    $(this).append('&nbsp;<a class="tag" style="display: none;" href="#' + $(this).attr('id') + '" name="' + $(this).attr('id') + '">¶</a>');
});
$('h2').hover(function() {
    $(this).children('.tag').css('display', 'inline');
}, function() {
    $(this).children('.tag').css('display', 'none');
});
