$.ajax('/assets/serverstatus/moneys.json', {
    dataType: 'json',
    error: function(request, status, error) {
        $('.funding-month').html('(error)');
        $('.funding-progressbar').removeClass('active');
        $('.funding-progressbar').children('.progress-bar').addClass('progress-bar-danger');
    },
    success: function(data) {
        $('.funding-progressbar').removeClass('active progress-striped');
        $('.funding-progressbar').empty();
        var funding_total = 0.0;
        data['history'].forEach(function(transaction) {
            if (transaction['type'] != 'nessus-monthly') {
                funding_total += transaction['amount'];
            }
        });
        var year = 2013;
        var month = 9;
        while (funding_total >= Math.abs(data['spending_monthly'])) {
            month++;
            if (month > 12) {
                year++;
                month = 1;
            }
            funding_total -= Math.abs(data['spending_monthly']);
        }
        var months = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
        if (month == 12) {
            $('.funding-month').html('December ' + year + ' to January ' + (year + 1));
        } else {
            $('.funding-month').html(months[month - 1] + ' to ' + months[month] + ' ' + year);
        }
        var percent = 0;
        if (funding_total > 0.0) {
            percent = Math.floor(funding_total * 100 / data['spending_monthly']);
            $('.funding-progressbar').append('<div class="progress-bar progress-bar-success" style="width: ' + percent + '%;"><span class="sr-only">' + percent + '% funded</span></div>');
        }
        if (percent < 100) {
            var expected_total = data['funding_monthly'];
            data['history'].forEach(function(transaction) {
                if (transaction['type'] == 'player-monthly') {
                    var transaction_year = transaction['date'].split('-')[0];
                    var transaction_month = transaction['date'].split('-')[1];
                    var transaction_day = transaction['date'].split('-')[2];
                    if (transaction_day < data['billing_dom']) {
                        if ((transaction_month - 1 == month && transaction_year == year) || (month == 12 && transaction_month == 1 && transaction_year - 1 == year)) {
                            expected_total -= transaction['amount'];
                        }
                    } else if (transaction_year == year && transaction_month == month) {
                        expected_total -= transaction['amount'];
                    }
                }
            });
            var expected_percent = Math.max(0, Math.min(100 - percent, Math.floor(expected_total * 100 / data['spending_monthly'])));
            $('.funding-progressbar').append('<div class="progress-bar progress-bar-warning" style="width: ' + expected_percent + '%;"><span class="sr-only">' + expected_percent + '% expected</span></div>');
        }
    }
});
