/// 数据大小单位换算,保留两位小数,如 1.20MB
pub fn to_data_size(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;
    let size = size as f64;
    let (value, unit) = if size >= TB {
        (size / TB, "TB")
    } else if size >= GB {
        (size / GB, "GB")
    } else if size >= MB {
        (size / MB, "MB")
    } else if size >= KB {
        (size / KB, "KB")
    } else {
        (size, "B")
    };
    format!("{:.2}{}", value, unit)
}

/// 将秒数转换成时间格式: 00:05、01:05、01:01:05
pub fn to_time_format(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let h = seconds / 3600;
    let m = seconds % 3600 / 60;
    let s = seconds % 60;
    if seconds >= 3600 {
        format!("{:02}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_size() {
        assert_eq!(to_data_size(0), "0.00B");
        assert_eq!(to_data_size(1023), "1023.00B");
        assert_eq!(to_data_size(1024), "1.00KB");
        assert_eq!(to_data_size(1024 * 1024 * 12 / 10), "1.20MB");
        assert_eq!(to_data_size(1024 * 1024 * 1024 * 3), "3.00GB");
    }

    #[test]
    fn time_format() {
        assert_eq!(to_time_format(5), "00:05");
        assert_eq!(to_time_format(65), "01:05");
        assert_eq!(to_time_format(3665), "01:01:05");
    }
}
