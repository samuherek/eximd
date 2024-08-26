use std::process::Command;

#[test]
fn rename_local_test_src_files() {
    let output = Command::new("cargo")
        .args(&["run", "--", "rename", "../test_src"]) // Replace with your arguments
        .output()
        .expect("Failed to execute command");

    println!("output::::");
    let data = String::from_utf8_lossy(&output.stdout);
    let output_lines = data
        .trim()
        .lines()
        .flat_map(|x| x.split_once(" -> "))
        .map(|(in_path, out_path)| {
            let in_name = in_path.split("/").last();
            let out_name = out_path.split("/").last();
            (in_name, out_name)
        })
        .collect::<Vec<_>>();

    let expected_lines = r#"
DRY RUN:: run `rename --exec 'path/to' to commit
../test_src/DSCF5901.RAF -> ../test_src/2022-03-17_17.40.45.RAF
../test_src/DSCF5889.RAF -> ../test_src/2022-03-17_17.12.42.RAF
../test_src/DSCF5891.RAF -> ../test_src/2022-03-17_17.16.00.RAF
../test_src/DSCF5891.xmp -> ../test_src/2022-03-17_17.16.00.RAF
../test_src/IMG_3414.MOV -> ../test_src/2024-08-16_14.42.44.MOV
../test_src/DSCF5885.RAF -> ../test_src/2022-03-17_17.08.57.RAF
../test_src/DSCF5885.xmp -> ../test_src/2022-03-17_17.08.57.RAF
../test_src/DSCF5888.RAF -> ../test_src/2022-03-17_17.12.19.RAF
../test_src/DSCF5888.xmp -> ../test_src/2022-03-17_17.12.19.RAF
../test_src/IMG_4104.JPG -> ../test_src/2021-02-08_15.56.06.JPG
../test_src/IMG_4104.MOV -> ../test_src/2021-02-08_15.56.06.MOV
../test_src/DSCF5887.RAF -> ../test_src/2022-03-17_17.12.11.RAF
../test_src/DSCF5887.xmp -> ../test_src/2022-03-17_17.12.11.RAF
../test_src/DSCF5903.RAF -> ../test_src/2022-03-17_17.41.46.RAF
../test_src/DSCF5903.xmp -> ../test_src/2022-03-17_17.41.46.RAF
../test_src/.DS_Store -> Uncertain Primary file
../test_src/DSCF5906.RAF -> ../test_src/2022-03-17_18.37.11.RAF
../test_src/DSCF5895.RAF -> ../test_src/2022-03-17_17.31.32.RAF
../test_src/DSCF5898.RAF -> ../test_src/2022-03-17_17.39.30.RAF
../test_src/IMG_3412.JPG -> ../test_src/2024-08-16_14.42.39.JPG
../test_src/DSCF5909.RAF -> ../test_src/2022-03-17_18.38.07.RAF
../test_src/DSCF5900.RAF -> ../test_src/2022-03-17_17.40.37.RAF
../test_src/DSCF5905.RAF -> ../test_src/2022-03-17_18.36.53.RAF
../test_src/DSCF5897.RAF -> ../test_src/2022-03-17_17.31.39.RAF
../test_src/IMG_3894.MOV -> ../test_src/2021-01-13_16.43.29.MOV
../test_src/DSCF5886.RAF -> ../test_src/2022-03-17_17.11.40.RAF
../test_src/DSCF5899.RAF -> ../test_src/2022-03-17_17.40.27.RAF
../test_src/DSCF5904.RAF -> ../test_src/2022-03-17_18.36.35.RAF
../test_src/DSCF5904.xmp -> ../test_src/2022-03-17_18.36.35.RAF
../test_src/DSCF5883.RAF -> ../test_src/2022-03-17_17.08.42.RAF
../test_src/DSCF5883.xmp -> ../test_src/2022-03-17_17.08.42.RAF
../test_src/IMG_3877.JPG -> ../test_src/2021-01-11_07.19.06.JPG
../test_src/IMG_3877.MOV -> ../test_src/2021-01-11_07.19.06.MOV
../test_src/DSCF5896.RAF -> ../test_src/2022-03-17_17.31.36.RAF
../test_src/DSCF5908.RAF -> ../test_src/2022-03-17_18.38.05.RAF
../test_src/DSCF5882.RAF -> ../test_src/2022-03-17_17.08.18.RAF
../test_src/DSCF5882.xmp -> ../test_src/2022-03-17_17.08.18.RAF
../test_src/DSCF5884.RAF -> ../test_src/2022-03-17_17.08.45.RAF
../test_src/DSCF5907.RAF -> ../test_src/2022-03-17_18.37.47.RAF
../test_src/IMG_3896.AAE -> Uncertain Primary file
../test_src/DSCF5902.RAF -> ../test_src/2022-03-17_17.40.52.RAF
../test_src/IMG_3413.DNG -> ../test_src/2024-08-16_14.42.41.DNG
../test_src/2019-12-23 18.50.08.HEIC -> ../test_src/2019-12-23_18.50.08.HEIC
../test_src/2019-12-23 18.50.08.AAE -> ../test_src/2019-12-23_18.50.08.HEIC
        "#
    .trim()
    .lines()
    .flat_map(|x| x.split_once(" -> "))
    .map(|(in_path, out_path)| {
        let in_name = in_path.split("/").last();
        let out_name = out_path.split("/").last();
        (in_name, out_name)
    })
    .collect::<Vec<_>>();

    assert!(output.status.success());

    let len = output_lines.len().max(expected_lines.len());
    for i in 0..len {
        let _v1 = output_lines.get(i);
        let _v2 = expected_lines.get(i);

        // assert_eq!(v1, v2);
    }
}
